mod imp;

use glib::Object;
use gtk::glib::clone;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use gtk::{prelude::*, ListItem};
use tracing::{error, info};

use crate::LYRIC_PROVIDERS;

glib::wrapper! {
    pub struct ResultObject(ObjectSubclass<imp::ResultObject>);
}

impl ResultObject {
    pub fn new(id: String, name: String) -> Self {
        Object::builder()
            .property("name", name)
            .property("id", id)
            .build()
    }
}

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Window {
    pub fn new() -> Self {
        let window: Self = Object::builder().build();
        window.set_title(Some("Search lyric"));
        window
    }

    fn results(&self) -> gio::ListStore {
        self.imp()
            .results
            .borrow()
            .clone()
            .expect("Could not get results")
    }

    fn setup_results(&self) {
        let model = gio::ListStore::new(ResultObject::static_type());

        self.imp().results.replace(Some(model));

        let selection_model = gtk::SingleSelection::new(Some(self.results()));
        self.imp().result_list.set_model(Some(&selection_model));
    }

    fn setup_ui(&self) {
        let imp = self.imp();

        imp.vbox
            .set_properties(&[("orientation", &gtk::Orientation::Vertical)]);
        imp.vbox.append(&imp.input);
        imp.vbox.append(&imp.result_list);
        imp.vbox.append(&imp.set_button);

        imp.input.set_placeholder_text(Some("Enter query..."));
        imp.input
            .set_secondary_icon_name(Some("system-search-symbolic"));
        imp.result_list.set_height_request(200);
        imp.set_button.set_label("Set as lyric");

        self.set_child(Some(&imp.vbox));
    }

    fn search(&self) {
        let buffer = self.imp().input.buffer();
        let query = buffer.text().to_string();
        if query.is_empty() {
            return;
        }

        let mut results = vec![];
        LYRIC_PROVIDERS.with_borrow(|providers| {
            for provider in providers {
                let provider_id = provider.provider_unique_name();
                // Use a hack to directly search with query
                let tracks =
                    match crate::sync::player::search_song(provider.as_ref(), "", &[""], &query) {
                        Ok(songs) => songs,
                        Err(e) => {
                            // TODO: to show errors to users in GUI
                            error!("{e} occurs when search {query} on {}", provider_id);
                            continue;
                        }
                    };
                for track in tracks {
                    // TODO: present in a better format
                    results.push(ResultObject::new(
                        track.id,
                        format!(
                            "{} {} {:?} ({}s)",
                            track.title,
                            track.singer,
                            track.album,
                            track.length.as_secs()
                        ),
                    ));
                }
            }
        });
        self.results().remove_all();
        for result in results {
            self.results().append(&result);
        }
    }

    fn get_selected_result(&self) -> Option<ResultObject> {
        let selection_model = self
            .imp()
            .result_list
            .model()
            .and_downcast::<gtk::SingleSelection>()
            .expect("Needs to be SingleSelection");
        let selected = selection_model.selected();
        if selected == gtk::INVALID_LIST_POSITION {
            return None;
        }
        let result = self
            .results()
            .item(selected)
            .and_downcast::<ResultObject>()
            .expect("Needs to be ResultObject");
        Some(result)
    }

    fn setup_callbacks(&self) {
        let imp = self.imp();
        imp.input
            .connect_activate(clone!(@weak self as window => move |_| {
                window.search();
            }));

        imp.input
            .connect_icon_release(clone!(@weak self as window => move |_, _| {
                window.search();
            }));

        imp.set_button
            .connect_clicked(clone!(@weak self as window => move |_| {
                // TODO: set lyric
                let result = window.get_selected_result();
                info!("set lyric: {:?}", result);
            }));
    }

    fn setup_factory(&self) {
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            let label = gtk::Label::new(None);
            list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .set_child(Some(&label));
        });
        factory.connect_bind(move |_, list_item| {
            let result_object = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .item()
                .and_downcast::<ResultObject>()
                .expect("Needs to be ResultObject");
            let label = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<gtk::Label>()
                .expect("Needs to be Label");

            label.set_label(&result_object.name());
        });
        self.imp().result_list.set_factory(Some(&factory));
    }
}