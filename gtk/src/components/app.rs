//! Application window.

use std::cell::RefCell;
use std::fs::File;
use std::io::BufReader;
use std::mem::replace;
use std::path::Path;
use std::rc::Rc;

use gtk::*;
use rayon::prelude::*;

use fimfareader::archive::Fetcher;
use fimfareader::archive::Story;
use fimfareader::query::parse;

pub enum Store {
    Attached(StoryView),
    Detached(TreeView),
    Empty,
}

pub struct StoryView {
    fetcher: Fetcher<BufReader<File>>,
    filter: TreeModelFilter,
    rows: Vec<TreeIter>,
    store: ListStore,
    view: Option<TreeView>,
    visible: Vec<bool>,
}

pub struct AppWindow {
    query: Entry,
    store: Store,
    window: ApplicationWindow,
    open: FileChooserButton,
}

impl StoryView {
    pub fn new(fetcher: Fetcher<BufReader<File>>) -> Self {
        const LENGTH: usize = 4;

        let types: [Type; LENGTH] = [
            i64::static_type(),
            bool::static_type(),
            String::static_type(),
            String::static_type(),
        ];

        let store = ListStore::new(&types);
        let filter = TreeModelFilter::new(&store, None);
        let columns: [u32; LENGTH] = [0, 1, 2, 3];
        let view = None;

        let mut rows: Vec<TreeIter> = Vec::with_capacity(fetcher.len());
        let mut visible: Vec<bool> = Vec::with_capacity(fetcher.len());

        for (i, story) in fetcher.iter().enumerate() {
            let row = Some(i as u32);

            let values: [&dyn ToValue; LENGTH] =
                [&story.id, &true, &story.title, &story.author.name];

            let row = store.insert_with_values(row, &columns, &values);

            visible.push(true);
            rows.push(row);
        }

        filter.set_visible_column(1);

        StoryView {
            fetcher,
            filter,
            rows,
            store,
            view,
            visible,
        }
    }

    pub fn attach(&mut self, view: TreeView) -> Option<TreeView> {
        view.set_model(Some(&self.filter));

        self.view.replace(view).and_then(|view| {
            view.set_model(None::<&TreeModel>);
            Some(view)
        })
    }

    pub fn detach(&mut self) -> Option<TreeView> {
        self.view.take().and_then(|view| {
            view.set_model(None::<&TreeModel>);
            Some(view)
        })
    }

    pub fn filter<T>(&mut self, filter: T)
    where
        T: Sync + Fn(&Story) -> bool,
    {
        let store = &self.store;
        let visible = &mut self.visible;

        let bitmap: Vec<bool> = self.fetcher.par_iter().map(&filter).collect();

        for (i, row) in self.rows.iter().enumerate() {
            let new = bitmap[i];
            let old = visible[i];

            if new != old {
                store.set_value(row, 1, &new.to_value());
            }
        }

        self.visible = bitmap;
    }
}

impl Drop for StoryView {
    fn drop(&mut self) {
        println!("DROP!");
        self.detach();
    }
}

impl AppWindow {
    pub fn new() -> Option<Rc<RefCell<Self>>> {
        let ui = include_str!("app.ui");
        let builder = Builder::new_from_string(ui);
        let result = builder.get_object("result")?;

        let this = Rc::new(RefCell::new(Self {
            query: builder.get_object("entry")?,
            store: Store::Detached(result),
            window: builder.get_object("app")?,
            open: builder.get_object("open")?,
        }));

        Self::connect(this.clone());

        Some(this)
    }

    fn connect(wrapper: Rc<RefCell<Self>>) {
        let this = wrapper.borrow();

        this.window.connect_destroy(move |_| gtk::main_quit());

        let clone = wrapper.clone();
        this.query.connect_activate(move |entry| {
            let mut this = clone.borrow_mut();
            let text = entry.get_text().unwrap();

            this.filter(text.as_str().trim());
        });

        let clone = wrapper.clone();
        this.open.connect_file_set(move |dialog| {
            let mut this = clone.borrow_mut();
            let path = dialog.get_filename().unwrap();

            this.load(path);
        });
    }

    pub fn load(&mut self, path: impl AsRef<Path>) {
        use Store::*;

        let view = match replace(&mut self.store, Empty) {
            Empty => panic!(),
            Detached(view) => view,
            Attached(mut store) => {
                let view = store.detach();
                drop(store);
                view.unwrap()
            }
        };

        let fetcher = Fetcher::from(path).unwrap();
        let mut store = StoryView::new(fetcher);

        store.attach(view);

        self.store = Attached(store);
    }

    pub fn filter(&mut self, query: &str) {
        use Store::*;

        let model = match &mut self.store {
            Attached(model) => model,
            Detached(_) => return,
            Empty => panic!(),
        };

        if query.trim() == "" {
            model.filter(|_| true);
            return;
        }

        match parse(query) {
            Err(_) => model.filter(|_| false),
            Ok(filter) => model.filter(filter),
        }
    }
}
