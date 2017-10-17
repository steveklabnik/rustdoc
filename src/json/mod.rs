//! Code used to serialize crate data to JSON.

mod api;

pub use self::api::*;

use analysis::{AnalysisHost, DefKind};

use error::*;

use std::collections::VecDeque;

/// Creates the documentation from the given `AnalysisHost`.
///
/// The documentation can be serialized to JSON.
pub fn create_documentation(host: &AnalysisHost, crate_name: &str) -> Result<Documentation> {
    // This function does a lot, so here's the plan:
    //
    // First, we need to process the root def and get its list of children.
    // Then, we process all of the children. Children may produce more children
    // to be processed too. Once we've processed them all, we're done.

    // Step one: we need to get all of the "def roots", and then find the
    // one that's our crate.
    let roots = host.def_roots()?;

    let id = roots.iter().find(|&&(_, ref name)| name == crate_name);
    let root_id = match id {
        Some(&(id, _)) => id,
        _ => return Err(ErrorKind::CrateErr(crate_name.to_string()).into()),
    };

    let root_def = host.get_def(root_id)?;

    // Create the main `Document`.
    let mut document = Document::new()
        .ty(String::from("crate"))
        .id(crate_name.to_string())
        .attributes(String::from("docs"), root_def.docs);

    // Now that we have that, it's time to get the children; these are
    // the top-level items for the crate.
    let ids = host.for_each_child_def(root_id, |id, _def| id).unwrap();


    // Now, we push all of those children onto a channel. The channel functions
    // as a work queue; we take an item off, process it, and then if it has
    // children, push them onto the queue. When the queue is empty, we've processed
    // everything.
    //
    // Additionally, we generate relationships between the crate itself and
    // these ids, as they're at the top level and hence linked with the crate.

    let mut queue = VecDeque::new();

    for id in ids {
        queue.push_back(id);

        let def = host.get_def(id).unwrap();

        let (ty, child_ty) = match def.kind {
            DefKind::Mod => (String::from("module"), String::from("modules")),
            DefKind::Struct => (String::from("struct"), String::from("structs")),
            _ => continue,
        };

        let data = Data::new().ty(ty.clone()).id(def.qualname.clone());

        document.add_relationship(child_ty, data);
    }

    // The loop below is basically creating this vector.
    let mut included: Vec<Document> = Vec::new();

    while let Some(id) = queue.pop_front() {
        // push each child to be processed itself, and also record
        // their ids so we can create the relationships for later
        let child_ids = host.for_each_child_def(id, |id, _def| {
            queue.push_back(id);
            id
        })?;

        // Question: we could do this by cloning it in the call to for_each_child_def
        // above/below; is that cheaper, or is this cheaper?
        let def = host.get_def(id).unwrap();

        // Using the item's metadata we create a new `Document` type to be put in the eventual
        // serialized JSON.
        let (ty, child_ty) = match def.kind {
            DefKind::Mod => (String::from("module"), String::from("child_modules")),
            DefKind::Struct => (String::from("struct"), String::from("child_structs")),
            _ => continue,
        };

        let mut document = Document::new()
            .ty(ty.clone())
            .id(def.qualname.clone())
            .attributes(String::from("name"), def.name)
            .attributes(String::from("docs"), def.docs);

        for id in child_ids {
            let def = host.get_def(id).unwrap();

            let data = Data::new().ty(ty.clone()).id(def.qualname.clone());

            document.add_relationship(child_ty.clone(), data);
        }

        included.push(document);
    }

    Ok(Documentation::new().data(document).included(included))
}
