//! `GObject` wrapper for `SearchResult` data.
//!
//! Required for GTK4's model-based widgets (`ListView`, `GridView`) which need
//! `GObject`-based items in `gio::ListStore`.

use gtk4::glib;
use gtk4::subclass::prelude::*;
use hamr_rpc::SearchResult;
use hamr_types::ResultType;
use std::cell::RefCell;

mod imp {
    use super::{ObjectImpl, ObjectSubclass, RefCell, SearchResult, glib};

    #[derive(Debug, Default)]
    pub struct ResultObjectInner {
        pub data: RefCell<Option<SearchResult>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ResultObjectInner {
        const NAME: &'static str = "HamrResultObject";
        type Type = super::ResultObject;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for ResultObjectInner {}
}

glib::wrapper! {
    pub struct ResultObject(ObjectSubclass<imp::ResultObjectInner>);
}

impl ResultObject {
    pub fn new(result: SearchResult) -> Self {
        let obj: Self = glib::Object::builder().build();
        obj.imp().data.replace(Some(result));
        obj
    }

    pub fn data(&self) -> Option<SearchResult> {
        self.imp().data.borrow().clone()
    }

    pub fn set_data(&self, result: SearchResult) {
        self.imp().data.replace(Some(result));
    }

    pub fn id(&self) -> String {
        self.imp()
            .data
            .borrow()
            .as_ref()
            .map(|r| r.id.clone())
            .unwrap_or_default()
    }

    pub fn name(&self) -> String {
        self.imp()
            .data
            .borrow()
            .as_ref()
            .map(|r| r.name.clone())
            .unwrap_or_default()
    }

    pub fn icon(&self) -> String {
        self.imp().data.borrow().as_ref().map_or_else(
            || "extension".to_string(),
            |r| r.icon_or_default().to_string(),
        )
    }

    pub fn icon_type(&self) -> Option<String> {
        self.imp()
            .data
            .borrow()
            .as_ref()
            .and_then(|r| r.icon_type.clone())
    }

    pub fn thumbnail(&self) -> Option<String> {
        self.imp()
            .data
            .borrow()
            .as_ref()
            .and_then(|r| r.thumbnail.clone())
    }

    pub fn verb(&self) -> String {
        self.imp()
            .data
            .borrow()
            .as_ref()
            .map_or_else(|| "Select".to_string(), |r| r.verb_or_default().to_string())
    }

    pub fn result_type(&self) -> ResultType {
        self.imp()
            .data
            .borrow()
            .as_ref()
            .map(|r| r.result_type)
            .unwrap_or_default()
    }

    pub fn actions(&self) -> Vec<hamr_types::Action> {
        self.imp()
            .data
            .borrow()
            .as_ref()
            .map(|r| r.actions.clone())
            .unwrap_or_default()
    }
}

impl Default for ResultObject {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}
