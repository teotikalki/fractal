extern crate serde_json;

use std::collections::HashMap;
use std::fs::File;
use std::fs::remove_dir_all;
use std::io::prelude::*;
use gtk;

use types::RoomList;
use error::Error;

use fractal_api::util::cache_path;
use globals;

/* includes for avatar download */
use backend::BKCommand;
use std::sync::mpsc::Sender;
use std::sync::mpsc::channel;
use std::sync::mpsc::TryRecvError;

use types::Message;

use std::cell::RefCell;
use std::rc::Rc;
use widgets::AvatarData;

#[derive(Serialize, Deserialize)]
pub struct CacheData {
    pub since: String,
    pub rooms: RoomList,
    pub last_viewed_messages: HashMap<String, Message>,
    pub username: String,
    pub uid: String,
}


pub fn store(
    rooms: &RoomList,
    last_viewed_messages: HashMap<String, Message>,
    since: String,
    username: String,
    uid: String
) -> Result<(), Error> {
    let fname = cache_path("rooms.json")?;

    let mut cacherooms = rooms.clone();
    for r in cacherooms.values_mut() {
        let skip = match r.messages.len() {
            n if n > globals::CACHE_SIZE => n - globals::CACHE_SIZE,
            _ => 0,
        };
        r.messages = r.messages.iter().skip(skip).cloned().collect();
    }

    let data = CacheData {
        since: since,
        rooms: cacherooms,
        last_viewed_messages: last_viewed_messages,
        username: username,
        uid: uid,
    };

    let serialized = serde_json::to_string(&data)?;
    File::create(fname)?.write_all(&serialized.into_bytes())?;

    Ok(())
}

pub fn load() -> Result<CacheData, Error> {
    let fname = cache_path("rooms.json")?;

    let mut file = File::open(fname)?;
    let mut serialized = String::new();
    file.read_to_string(&mut serialized)?;

   let deserialized: CacheData = serde_json::from_str(&serialized)?;

   Ok(deserialized)
}

pub fn destroy() -> Result<(), Error> {
    let fname = cache_path("")?;
    remove_dir_all(fname).or_else(|_| Err(Error::CacheError))
}

/// this downloads a avatar and stores it in the cache folder
pub fn download_to_cache(backend: Sender<BKCommand>,
                         name: String,
                         data: Rc<RefCell<AvatarData>>) {
    let (tx, rx) = channel::<(String, String)>();
    let _ = backend.send(BKCommand::GetUserInfoAsync(name.clone(), Some(tx)));

    gtk::timeout_add(50, move || match rx.try_recv() {
        Err(TryRecvError::Empty) => gtk::Continue(true),
        Err(TryRecvError::Disconnected) => gtk::Continue(false),
        Ok(_resp) => {
            data.borrow_mut().redraw_pixbuf();
            gtk::Continue(false)
        }
    });
}
