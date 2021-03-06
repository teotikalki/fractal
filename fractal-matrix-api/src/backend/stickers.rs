extern crate serde_json;
extern crate md5;
extern crate chrono;

use self::chrono::prelude::*;

use std::thread;
use util::json_q;
use util::{scalar_url, client_url};
use url::Url;
use std::sync::{Arc, Mutex};
use backend::BackendData;

use globals;
//use std::thread;
use error::Error;

use backend::types::Backend;
use backend::types::BKResponse;
use backend::types::BKCommand;
use types::StickerGroup;
use types::Sticker;
use self::serde_json::Value as JsonValue;


/// Queries scalar.vector.im to list all the stickers
pub fn list(bk: &Backend) -> Result<(), Error> {
    let widget = bk.data.lock().unwrap().sticker_widget.clone();
    if widget.is_none() {
        get_sticker_widget_id(bk, BKCommand::ListStickers)?;
        return Ok(());
    }

    let widget_id = widget.unwrap();
    let data = vec![
        ("widget_type", "m.stickerpicker".to_string()),
        ("widget_id", widget_id),
        ("filter_unpurchased", "true".to_string()),
    ];
    let url = vurl(bk.data.clone(), "widgets/assets", data)?;

    let tx = bk.tx.clone();
    get!(&url,
        |r: JsonValue| {
            let mut stickers = vec![];
            for sticker_group in r["assets"].as_array().unwrap_or(&vec![]).iter() {
                let group = StickerGroup::from_json(sticker_group);
                stickers.push(group);
            }
            tx.send(BKResponse::Stickers(stickers)).unwrap();
        },
        |err| { tx.send(BKResponse::StickersError(err)).unwrap() }
    );

    Ok(())
}

pub fn get_sticker_widget_id(bk: &Backend, then: BKCommand) -> Result<(), Error> {
    let data = json!({
        "data": {},
        "type": "m.stickerpicker",
    });
    let d = bk.data.clone();
    let tx = bk.internal_tx.clone();

    thread::spawn(move || {
        let url = vurl(d.clone(), "widgets/request", vec![]).unwrap();
        match json_q("post", &url, &data, globals::TIMEOUT) {
            Ok(r) => {
                let mut id = "".to_string();
                if let Some(i) = r["id"].as_str() {
                    id = i.to_string();
                }
                if let Some(i) = r["data"]["id"].as_str() {
                    id = i.to_string();
                }

                let widget_id = match id.is_empty() {
                    true => None,
                    false => Some(id),
                };
                d.lock().unwrap().sticker_widget = widget_id;
            },
            Err(Error::MatrixError(js)) => {
                let widget_id = js["data"]["id"].as_str().map(|id| id.to_string());
                d.lock().unwrap().sticker_widget = widget_id;
            },
            Err(_err) => {
                d.lock().unwrap().sticker_widget = None;
            }
        };
        if let Some(t) = tx {
            t.send(then).unwrap();
        }
    });

    Ok(())
}

pub fn send(bk: &Backend, roomid: String, sticker: &Sticker) -> Result<(), Error> {
    let now = Local::now();
    let msg = format!("{}{}{}", roomid, sticker.name, now.to_string());
    let digest = md5::compute(msg.as_bytes());
    // TODO: we need to generate the msg.id in the frontend
    let id = format!("{:x}", digest);

    let url = bk.url(&format!("rooms/{}/send/m.sticker/{}", roomid, id), vec![])?;

    let attrs = json!({
        "body": sticker.body.clone(),
        "url": sticker.url.clone(),
        "info": {
            "w": sticker.size.0,
            "h": sticker.size.1,
            "thumbnail_url": sticker.thumbnail.clone(),
        },
    });

    let tx = bk.tx.clone();
    query!("put", &url, &attrs,
        move |js: JsonValue| {
            let evid = js["event_id"].as_str().unwrap_or_default();
            tx.send(BKResponse::SentMsg(id, evid.to_string())).unwrap();
        },
        |_| { tx.send(BKResponse::SendMsgError(Error::SendMsgError(id))).unwrap(); }
    );

    Ok(())
}

pub fn purchase(bk: &Backend, group: &StickerGroup) -> Result<(), Error> {
    let widget = bk.data.lock().unwrap().sticker_widget.clone();
    if widget.is_none() {
        get_sticker_widget_id(bk, BKCommand::PurchaseSticker(group.clone()))?;
        return Ok(());
    }

    let widget_id = widget.unwrap();
    let asset = group.asset.clone();
    let data = vec![
        ("asset_type", asset.clone()),
        ("widget_id", widget_id.clone()),
        ("widget_type", "m.stickerpicker".to_string()),
    ];
    let url = vurl(bk.data.clone(), "widgets/purchase_asset", data)?;
    let tx = bk.tx.clone();
    let itx = bk.internal_tx.clone();
    get!(&url,
        |_| {
            if let Some(t) = itx {
                t.send(BKCommand::ListStickers).unwrap();
            }
        },
        |err| { tx.send(BKResponse::StickersError(err)).unwrap() }
    );

    Ok(())
}

fn get_base_url(data: Arc<Mutex<BackendData>>) -> Result<Url, Error> {
    let s = data.lock().unwrap().server_url.clone();
    let url = Url::parse(&s)?;
    Ok(url)
}

fn url(data: Arc<Mutex<BackendData>>, path: &str, params: Vec<(&str, String)>) -> Result<Url, Error> {
    let base = get_base_url(data.clone())?;
    let tk = data.lock().unwrap().access_token.clone();

    let mut params2 = params.to_vec();
    params2.push(("access_token", tk.clone()));

    client_url(&base, path, params2)
}

fn get_scalar_token(data: Arc<Mutex<BackendData>>) -> Result<String, Error> {
    let s = data.lock().unwrap().scalar_url.clone();
    let uid = data.lock().unwrap().user_id.clone();

    let url = url(data.clone(), &format!("user/{}/openid/request_token", uid), vec![])?;
    let js = json_q("post", &url, &json!({}), globals::TIMEOUT)?;

    let vurl = Url::parse(&format!("{}/api/register", s))?;
    let js = json_q("post", &vurl, &js, globals::TIMEOUT)?;

    match js["scalar_token"].as_str() {
        Some(st) => {
            data.lock().unwrap().scalar_token = Some(st.to_string());
            Ok(st.to_string())
        }
        None => Err(Error::BackendError),
    }
}

fn vurl(data: Arc<Mutex<BackendData>>, path: &str, params: Vec<(&str, String)>) -> Result<Url, Error> {
    let s = data.lock().unwrap().scalar_url.clone();
    let base = Url::parse(&s)?;
    let token = data.lock().unwrap().scalar_token.clone();
    let tk = match token {
        None => get_scalar_token(data)?,
        Some(t) => t.clone(),
    };

    let mut params2 = params.to_vec();
    params2.push(("scalar_token", tk));

    scalar_url(&base, path, params2)
}

