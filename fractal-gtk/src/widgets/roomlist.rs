extern crate url;
extern crate gtk;

use self::url::Url;
use std::collections::HashMap;
use self::gtk::prelude::*;

use widgets::roomrow::RoomRow;
use types::Room;
use types::Message;


fn get_url(url: Option<String>) -> Url {
    let defurl = Url::parse("https://matrix.org").unwrap();

    match url {
        Some(u) => {
            match Url::parse(&u) {
                Ok(url) => url,
                Err(_) => defurl,
            }
        }
        None => defurl,
    }
}


pub struct RoomList {
    pub rooms: HashMap<String, RoomRow>,
    pub baseu: Url,
    list: gtk::ListBox,

    roomvec: Vec<Room>,
    // TODO:
    // * Add a header to the list
    // * Add a collapse/expand button with a revealer
    // * Add drag & drop support for favorites
}

impl RoomList {
    pub fn new(url: Option<String>) -> RoomList {
        let list = gtk::ListBox::new();
        let baseu = get_url(url);
        let rooms = HashMap::new();
        let roomvec = vec![];

        RoomList {
            list,
            baseu,
            rooms,
            roomvec,
        }
    }

    pub fn add_room(&mut self, r: Room) {
        if self.rooms.contains_key(&r.id) {
            // room added, we'll pass
            return;
        }

        let rid = r.id.clone();
        self.roomvec.push(r.clone());

        let row = RoomRow::new(r, &self.baseu);
        self.list.add(&row.widget());

        self.rooms.insert(rid, row);
    }

    pub fn prepend_room(&mut self, r: Room) {
        if self.rooms.contains_key(&r.id) {
            // room added, we'll pass
            return;
        }

        let rid = r.id.clone();
        self.roomvec.insert(0, r.clone());

        let row = RoomRow::new(r, &self.baseu);
        self.list.prepend(&row.widget());

        self.rooms.insert(rid, row);
    }

    pub fn set_room_notifications(&mut self, room: String, n: i32) {
        if let Some(r) = self.rooms.get(&room) {
            r.set_notifications(n);
        }

        if let Some(ref mut rv) = self.room_from_v(&room) {
            rv.notifications = n;
        }
    }

    pub fn remove_room(&mut self, room: String) -> Option<Room> {
        self.rooms.remove(&room);
        if let Some(idx) = self.roomvec.iter().position(|x| { x.id == room}) {
            if let Some(row) = self.list.get_row_at_index(idx as i32) {
                self.list.remove(&row);
            }
            return Some(self.roomvec.remove(idx));
        }

        None
    }

    pub fn rename_room(&mut self, room: String, newname: Option<String>) {
        if let (Some(r), Some(n)) = (self.rooms.get_mut(&room), newname.clone()) {
            r.set_name(n);
        }

        if let Some(ref mut rv) = self.room_from_v(&room) {
            rv.name = newname;
        }
    }

    pub fn set_room_avatar(&mut self, room: String, av: Option<String>) {
        if let Some(r) = self.rooms.get_mut(&room) {
            r.set_avatar(av.clone());
        }

        if let Some(ref mut rv) = self.room_from_v(&room) {
            rv.avatar = av;
        }
    }

    pub fn widget(&self) -> gtk::Box {
        let b = gtk::Box::new(gtk::Orientation::Vertical, 0);
        if let Some(style) = b.get_style_context() {
            style.add_class("room-list");
        }

        b.pack_start(&self.list, true, true, 0);
        b.show_all();
        self.render_notifies();

        b
    }

    pub fn connect<F: Fn(Room) + 'static>(&self, cb: F) {
        let rs = self.roomvec.clone();
        self.list.connect_row_activated(move |_, row| {
            let idx = row.get_index();
            cb(rs[idx as usize].clone());
        });
    }

    pub fn get_selected(&self) -> Option<String> {
        match self.list.get_selected_row() {
            Some(row) => Some(self.roomvec[row.get_index() as usize].id.clone()),
            None => None,
        }
    }

    pub fn set_selected(&self, room: Option<String>) {
        self.list.unselect_all();

        if room.is_none() {
            return;
        }

        let room = room.unwrap();

        if let Some(idx) = self.roomvec.iter().position(|x| { x.id == room}) {
            if let Some(ref row) = self.list.get_row_at_index(idx as i32) {
                self.list.select_row(row);
            }
        }
    }

    pub fn add_rooms(&mut self, mut array: Vec<Room>) {
        array.sort_by_key(|ref x| {
            match x.messages.last() {
                Some(l) => l.date,
                None => Message::default().date,
            }
        });

        for r in array.iter().rev() {
            self.add_room(r.clone());
        }
    }

    pub fn moveup(&mut self, room: String) {
        let s = self.get_selected();

        if let Some(r) = self.remove_room(room) {
            self.prepend_room(r);
        }

        self.set_selected(s);
    }

    fn render_notifies(&self) {
        for (_k, r) in self.rooms.iter() {
            r.render_notifies();
        }
    }

    fn room_from_v(&mut self, room: &str) -> Option<&mut Room> {
        if let Some(idx) = self.roomvec.iter().position(|x| { x.id == room}) {
            return self.roomvec.get_mut(idx);
        }
        None
    }
}