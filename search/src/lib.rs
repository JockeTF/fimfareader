//! Main module.

use std::cell::RefCell;
use std::io::Cursor;
use std::io::Read;
use std::io::Seek;
use std::path::Path;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Instant;

use rayon::iter::ParallelIterator;
use zip::read::ZipArchive;

use tantivy::collector::TopDocs;
use tantivy::directory::MmapDirectory;
use tantivy::query::QueryParser;
use tantivy::schema;
use tantivy::schema::Document;
use tantivy::schema::Schema;
use tantivy::schema::Value;
use tantivy::Index;
use tantivy::ReloadPolicy;

use thread_local::ThreadLocal;

use fimfareader::prelude::*;

pub struct Searcher {
    schema: Schema,
    index: Index,
}

impl Searcher {
    pub fn new<T, F>(fetcher: &Fetcher<T>, f: &F) -> Self
    where
        T: Read + Seek + Send,
        F: Fn() -> ZipArchive<T> + Sync,
    {
        let mut builder = Schema::builder();
        builder.add_i64_field("sid", schema::INDEXED | schema::STORED);
        builder.add_text_field("content", schema::TEXT);
        let schema = builder.build();

        let index = Self::load_index(schema.clone(), fetcher, f);

        Searcher { schema, index }
    }

    fn load_index<T, F>(schema: Schema, fetcher: &Fetcher<T>, f: &F) -> Index
    where
        T: Read + Seek + Send,
        F: Fn() -> ZipArchive<T> + Sync,
    {
        let identity = fetcher.identity().unwrap();
        let directory = Path::new("cache").join(identity);

        if !directory.exists() {
            Self::make_index(schema.clone(), fetcher, f);
        }

        let store = MmapDirectory::open(&directory).unwrap();
        return Index::open_or_create(store, schema).unwrap();
    }

    fn make_index<T, F>(schema: Schema, fetcher: &Fetcher<T>, f: &F)
    where
        T: Read + Seek + Send,
        F: Fn() -> ZipArchive<T> + Sync,
    {
        let identity = fetcher.identity().unwrap();
        let directory = Path::new("cache").join(identity);

        std::fs::create_dir_all(&directory).unwrap();
        let store = MmapDirectory::open(&directory).unwrap();
        let index = Index::create(store, schema).unwrap();

        let schema = index.schema();
        let sid = schema.get_field("sid").unwrap();
        let content = schema.get_field("content").unwrap();
        let mut writer = index.writer(536_870_912).unwrap();

        let counter = AtomicUsize::new(0);
        let total = fetcher.iter().count();
        let start = Instant::now();

        let local = ThreadLocal::new();

        fetcher.par_iter().for_each(|story| {
            let mut doc = Document::default();
            doc.add_i64(sid, story.id);

            let archive = local.get_or(|| RefCell::new(f()));

            let mut archive = archive.borrow_mut();
            let mut file = archive.by_name(&story.archive.path).unwrap();
            let mut data = Vec::with_capacity(file.size() as usize);
            let mut text = String::with_capacity(1_048_576);

            file.read_to_end(&mut data).unwrap();
            let mut arch = ZipArchive::new(Cursor::new(data)).unwrap();
            let count = counter.fetch_add(1, Ordering::SeqCst);

            let percentage = (count as f64 / total as f64) * 100f64;
            print!("\r\rIndexing archive... {:.2}%\r\r", percentage);

            for i in 0..arch.len() {
                let mut file = arch.by_index(i).unwrap();

                if !file.name().ends_with(".html") {
                    continue;
                }

                file.read_to_string(&mut text).unwrap();
                doc.add_text(content, &text);
                text.clear();
            }

            writer.add_document(doc);
        });

        writer.commit().unwrap();

        let finish = (Instant::now() - start).as_secs();
        println!("Index generated in {} seconds.", finish);
    }

    pub fn search(&self, text: &str) -> Vec<(i64, f32)> {
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)
            .try_into()
            .unwrap();

        let searcher = reader.searcher();
        let identitfier = self.schema.get_field("sid").unwrap();
        let content = self.schema.get_field("content").unwrap();
        let parser = QueryParser::for_index(&self.index, vec![content]);

        let limit = TopDocs::with_limit(32);
        let query = parser.parse_query(&text).unwrap();
        let docs = searcher.search(&query, &limit).unwrap();

        docs.into_iter()
            .map(|(score, address)| {
                let doc = searcher.doc(address).unwrap();

                match doc.get_first(identitfier) {
                    Some(Value::I64(value)) => (*value, score),
                    _ => panic!("Invalid story key type!"),
                }
            })
            .collect()
    }

    pub fn parse(&self, text: &str) -> impl Fn(&Story) -> bool + Sync {
        let mut sids: Vec<_> = self
            .search(text)
            .into_iter()
            .filter(|(_, score)| *score > 10f32)
            .map(|(sid, _)| sid)
            .collect();

        sids.sort();

        move |story| sids.binary_search(&story.id).is_ok()
    }
}
