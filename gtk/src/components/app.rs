//! Application window.

use std::fs::File;
use std::io::BufReader;
use std::rc::Rc;
use std::sync::Mutex;

use gtk::*;

use fimfareader::archive::Fetcher;
use fimfareader::archive::Story;
use fimfareader::query::parse;

pub struct AppWindow {
    fetcher: Fetcher<BufReader<File>>,
    window: ApplicationWindow,
    search: ToggleButton,
    result: TreeView,
    entry: Entry,
}

impl AppWindow {
    pub fn new(fetcher: Fetcher<BufReader<File>>) -> Option<Rc<Self>> {
        let ui = include_str!("app.ui");
        let builder = Builder::new_from_string(ui);

        let this = Rc::new(Self {
            fetcher: fetcher,
            window: builder.get_object("app")?,
            search: builder.get_object("search")?,
            result: builder.get_object("result")?,
            entry: builder.get_object("entry")?,
        });

        this.apply(this.fetcher.iter().collect());
        Some(Self::connect(this))
    }

    fn connect(self: Rc<Self>) -> Rc<Self> {
        self.window.connect_destroy(move |_| gtk::main_quit());

        let clone = self.clone();
        self.entry.connect_activate(move |entry| {
            let text = entry.get_text().unwrap();
            clone.filter(text.as_str().trim());
        });

        self
    }

    fn apply(&self, stories: Vec<&Story>) {
        let store = ListStore::new(&[
            i64::static_type(),
            String::static_type(),
            String::static_type(),
        ]);

        for (i, story) in stories.iter().enumerate() {
            store.insert_with_values(
                Some(i as u32),
                &[0, 1, 2],
                &[&story.id, &story.title, &story.author.name],
            );
        }

        self.result.set_model(Some(&store));
    }

    pub fn filter(&self, query: &str) {
        let filter = parse(query);

        if filter.is_err() {
            return;
        }

        let filter = filter.unwrap();
        let vector = self.fetcher.filter(&filter);

        self.apply(vector);
    }
}
