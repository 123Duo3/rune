use std::collections::HashMap;
use std::error::Error;

use deunicode::deunicode;
use log::warn;
use tantivy::collector::{FilterCollector, TopDocs};
use tantivy::doc;
use tantivy::query::QueryParser;
use tantivy::schema::*;

use crate::connection::SearchDbConnection;

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
pub enum CollectionType {
    Track,
    Artist,
    Directory,
    Album,
    Playlist,
}

impl From<CollectionType> for i64 {
    fn from(collection_type: CollectionType) -> Self {
        match collection_type {
            CollectionType::Track => 0,
            CollectionType::Artist => 1,
            CollectionType::Album => 2,
            CollectionType::Directory => 3,
            CollectionType::Playlist => 4,
        }
    }
}

impl TryFrom<i64> for CollectionType {
    type Error = &'static str;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(CollectionType::Track),
            1 => Ok(CollectionType::Artist),
            2 => Ok(CollectionType::Album),
            3 => Ok(CollectionType::Directory),
            4 => Ok(CollectionType::Playlist),
            _ => Err("Invalid value for CollectionType"),
        }
    }
}

pub fn remove_term(search_db: &mut SearchDbConnection, r#type: CollectionType, id: i32) {
    let schema = &search_db.schema;
    let term_tid = schema.get_field("tid").unwrap();

    let tid = format!("{:?}-{:?}", r#type, id);
    let term = Term::from_field_text(term_tid, &tid);

    search_db.w.delete_term(term);
}

pub fn add_term(search_db: &mut SearchDbConnection, r#type: CollectionType, id: i32, name: &str) {
    let schema = &search_db.schema;
    let term_name = schema.get_field("name").unwrap();
    let term_latinization = schema.get_field("latinization").unwrap();
    let term_id = schema.get_field("id").unwrap();
    let term_type = schema.get_field("type").unwrap();
    let term_tid = schema.get_field("tid").unwrap();

    let tid = format!("{:?}-{:?}", term_type, term_id);
    let term = Term::from_field_text(term_tid, &tid);

    search_db.w.delete_term(term);

    search_db
        .w
        .add_document(doc!(
            term_name => name,
            term_latinization => deunicode(name),
            term_type => Into::<i64>::into(r#type),
            term_tid => tid,
            term_id => Into::<i64>::into(id),
        ))
        .unwrap();
}

pub fn search_for(
    search_db: &mut SearchDbConnection,
    query_str: &str,
    n: usize,
) -> Result<HashMap<CollectionType, Vec<i64>>, Box<dyn Error>> {
    let schema = &search_db.schema;
    let term_name = schema.get_field("name").unwrap();
    let term_latinization = schema.get_field("latinization").unwrap();
    let field_id = schema.get_field("id").unwrap();

    let query_parser = QueryParser::for_index(&search_db.index, vec![term_name, term_latinization]);
    let query = query_parser.parse_query(query_str)?;

    let searcher = search_db.index.reader()?.searcher();

    let mut results: HashMap<CollectionType, Vec<i64>> = HashMap::new();

    for collection_type in [
        CollectionType::Track,
        CollectionType::Artist,
        CollectionType::Album,
        CollectionType::Directory,
        CollectionType::Playlist,
    ] {
        let type_value = i64::from(collection_type.clone());
        let filter_collector = FilterCollector::new(
            "type".to_string(),
            move |value: i64| value == type_value,
            TopDocs::with_limit(n),
        );

        let top_docs = searcher.search(&query, &filter_collector)?;

        for (_score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;
            if let Some(doc_id) = retrieved_doc.get_first(field_id) {
                results
                    .entry(collection_type.clone())
                    .or_default()
                    .push(doc_id.as_i64().unwrap());
            } else {
                warn!("Id not inserted while searching for the document");
            }
        }
    }

    Ok(results)
}
