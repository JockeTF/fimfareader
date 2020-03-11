//! Main module.

use std::cell::RefCell;
use std::env::args;
use std::fs::File;
use std::io::BufReader;
use std::io::Cursor;
use std::io::Read;
use std::io::Seek;
use std::path::Path;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Instant;

use rayon::iter::ParallelIterator;
use rustyline::Editor;
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

fn exit(error: Error) -> ! {
    eprintln!("{}", error);

    std::process::exit(1)
}

fn load_index<T>(schema: Schema, fetcher: &Fetcher<T>, path: &str) -> Index
where
    T: Read + Seek + Send,
{
    let identity = fetcher.identity().unwrap();
    let directory = Path::new("search").join(identity);

    if directory.exists() {
        let store = MmapDirectory::open(&directory).unwrap();
        return Index::open_or_create(store, schema).unwrap();
    }

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

        let archive = local.get_or(|| {
            let reader = BufReader::new(File::open(&path).unwrap());
            RefCell::new(ZipArchive::new(reader).unwrap())
        });

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

    index
}

fn main() {
    let argv = args().collect::<Vec<String>>();
    let mut editor = Editor::<()>::new();

    if argv.len() != 2 {
        eprintln!("Usage: fimfareader <ARCHIVE>");
        std::process::exit(1);
    }

    println!("Hellopaca, World!");

    let start = Instant::now();
    let result = Fetcher::new(&argv[1]);
    let fetcher = result.map_err(exit).unwrap();
    let finish = (Instant::now() - start).as_millis();
    let count = fetcher.iter().count();

    println!("Finished loading in {} milliseconds.", finish);
    println!("The archive contains {} stories.", count);

    let mut builder = Schema::builder();
    let sid = builder.add_i64_field("sid", schema::INDEXED | schema::STORED);
    let content = builder.add_text_field("content", schema::TEXT);
    let index = load_index(builder.build(), &fetcher, &argv[1]);

    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommit)
        .try_into()
        .unwrap();

    let searcher = reader.searcher();
    let parser = QueryParser::for_index(&index, vec![content]);

    while let Ok(line) = editor.readline(">>> ") {
        editor.add_history_entry(&line);

        let limit = TopDocs::with_limit(16);
        let query = parser.parse_query(&line).unwrap();
        let docs = searcher.search(&query, &limit).unwrap();

        for (score, address) in docs {
            let doc = searcher.doc(address).unwrap();

            let story = match doc.get_first(sid).unwrap() {
                Value::I64(value) => fetcher.fetch(*value).unwrap(),
                _ => panic!("Invalid story key type!"),
            };

            println!("{:02.0}% [{:06}] {}", score, story.id, story.title);
        }
    }
}
