//! Main module.

use std::fs::create_dir_all;
use std::io::stdout;
use std::io::Cursor;
use std::io::Read;
use std::io::Seek;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema;
use tantivy::schema::Schema;
use tantivy::schema::Value;
use tantivy::Index;
use tantivy::ReloadPolicy;
use tantivy::TantivyDocument;
use zip::read::ZipArchive;

use fimfareader::prelude::*;

pub struct Searcher {
    index: Index,
}

impl Searcher {
    pub fn new<T>(fetcher: &Fetcher<T>) -> Self
    where
        T: Read + Seek + Send,
    {
        Searcher {
            index: Self::load_index(fetcher),
        }
    }

    fn schema() -> Schema {
        let mut builder = Schema::builder();

        builder.add_i64_field("sid", schema::INDEXED | schema::STORED);
        builder.add_text_field("content", schema::TEXT);

        builder.build()
    }

    fn load_index<T>(fetcher: &Fetcher<T>) -> Index
    where
        T: Read + Seek + Send,
    {
        let identity = fetcher.identity().unwrap();
        let path = Path::new("cache").join(identity);

        if path.exists() {
            Index::open_in_dir(path).unwrap()
        } else {
            Self::make_index(&path, fetcher)
        }
    }

    fn make_index<T>(path: &Path, fetcher: &Fetcher<T>) -> Index
    where
        T: Read + Seek + Send,
    {
        let start = Instant::now();
        print!("\r\rIndexing archive...\r\r");
        create_dir_all(path).unwrap();

        let schema = Self::schema();
        let index = Index::create_in_dir(path, schema).unwrap();
        let mut writer = index.writer(1_073_741_824).unwrap();
        let mut buffer = String::with_capacity(1_048_576);

        let schema = index.schema();
        let identifier = schema.get_field("sid").unwrap();
        let content = schema.get_field("content").unwrap();
        let story_count = fetcher.iter().count() as f64;

        for (i, story) in fetcher.iter().enumerate() {
            let progress = (i * 100) as f64 / story_count;
            print!("\r\rIndexing archive... {progress:.2}%\r\r");

            let cursor = Cursor::new(fetcher.read(story).unwrap());
            let mut epub = ZipArchive::new(cursor).unwrap();
            let mut document = TantivyDocument::default();

            document.add_i64(identifier, story.id);

            for i in 0..epub.len() {
                let mut file = epub.by_index(i).unwrap();

                if !file.name().ends_with(".html") {
                    continue;
                }

                file.read_to_string(&mut buffer).unwrap();
                document.add_text(content, &buffer);
                buffer.clear();
            }

            writer.add_document(document).unwrap();
        }

        print!("\r\rCommitting archive index...\r\r");
        stdout().flush().unwrap();

        writer.commit().unwrap();
        writer.wait_merging_threads().unwrap();

        let finish = (Instant::now() - start).as_secs();
        println!("Index generated in {finish} seconds.");

        index
    }

    pub fn search(&self, text: &str) -> Vec<(i64, f32)> {
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .unwrap();

        let schema = self.index.schema();
        let identifier = schema.get_field("sid").unwrap();
        let content = schema.get_field("content").unwrap();

        let parser = QueryParser::for_index(&self.index, vec![content]);
        let query = parser.parse_query(text).unwrap();

        let searcher = reader.searcher();
        let limit = TopDocs::with_limit(32);
        let docs = searcher.search(&query, &limit).unwrap();

        docs.into_iter()
            .map(|(score, address)| {
                let doc: TantivyDocument = searcher.doc(address).unwrap();

                match doc.get_first(identifier).map(|v| v.as_i64()) {
                    Some(Some(value)) => (value, score),
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
