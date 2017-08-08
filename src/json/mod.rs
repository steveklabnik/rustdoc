//! Code used to serialize crate data to JSON.

mod api;

pub use self::api::*;

use std::collections::HashMap;

use analysis::{self, AnalysisHost, DefKind};
use rayon::prelude::*;
use serde_json;

use error::*;

/// This creates the JSON documentation from the given `AnalysisHost`.
pub fn create_json(host: &AnalysisHost, crate_name: &str) -> Result<String> {
    let roots = host.def_roots()?;

    let id = roots.iter().find(|&&(_, ref name)| name == crate_name);
    let root_id = match id {
        Some(&(id, _)) => id,
        _ => return Err(ErrorKind::CrateErr(crate_name.to_string()).into()),
    };

    let root_def = host.get_def(root_id)?;

    fn recur(id: &analysis::Id, host: &AnalysisHost) -> Vec<analysis::Def> {
        let mut ids = Vec::new();
        let mut defs = host.for_each_child_def(*id, |id, def| {
            ids.push(id);
            def.clone()
        }).unwrap();

        let child_defs: Vec<analysis::Def> = ids.into_par_iter()
            .map(|id: analysis::Id| recur(&id, host))
            .reduce(Vec::default, |mut a: Vec<analysis::Def>,
             b: Vec<analysis::Def>| {
                a.extend(b);
                a
            });
        defs.extend(child_defs);
        defs
    }

    let mut included: Vec<Document> = Vec::new();
    let mut relationships: HashMap<String, Vec<Data>> = HashMap::with_capacity(METADATA_SIZE);

    for def in recur(&root_id, host) {
        let (ty, relations_key) = match def.kind {
            DefKind::Mod => (String::from("module"), String::from("modules")),
            DefKind::Struct => (String::from("struct"), String::from("structs")),
            _ => continue,
        };

        // Using the item's metadata we create a new `Document` type to be put in the eventual
        // serialized JSON.
        included.push(
            Document::new()
                .ty(ty.clone())
                .id(def.qualname.clone())
                .attributes(String::from("name"), def.name)
                .attributes(String::from("docs"), def.docs),
        );

        let item_relationships = relationships.entry(relations_key).or_insert_with(
            Default::default,
        );
        item_relationships.push(Data::new().ty(ty).id(def.qualname));
    }

    let mut data_document = Document::new()
        .ty(String::from("crate"))
        .id(crate_name.to_string())
        .attributes(String::from("docs"), root_def.docs);

    // Insert all of the different types of relationships into this `Document` type only
    for (ty, data) in relationships {
        data_document.relationships(ty, data);
    }

    Ok(serde_json::to_string(
        &Documentation::new().data(data_document).included(
            included,
        ),
    )?)
}
