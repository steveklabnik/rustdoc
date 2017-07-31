//! Code used to serialize crate data to JSON. We use a subset of the JSON-API spec.

use std::collections::HashMap;

// Sizes for the HashMaps to avoid reallocation and large HashMap sizes when we know
// the upper limit for them

/// Number of possible values in the enum item::Metadata
pub const METADATA_SIZE: usize = 14;

/// Under each item in data.relationships how many fields are there. For now we only have one which
/// is data. As we might add more this number should increase
const DATA_SIZE: usize = 1;

/// The data structure used to serialize crate documentation data to JSON for consumption by the
/// front end. When fully serialized the data looks something like:
///
/// ```json
/// {
///     "data": {
///         "type": "crate",
///         "id": "example",
///         "attributes": {
///             "docs": "docs go here"
///         },
///         "relationships": {
///             "modules": {
///                 "data": [
///                     {
///                         "type": "module",
///                         "id": "example::example_module"
///                     }
///                 ]
///             }
///         }
///     },
///     "included": [
///         {
///             "type": "module",
///             "id": "example::example_module",
///             "attributes": {
///                 "docs": "docs go here",
///                 "name": "example_module"
///             }
///         }
///     ]
///}
///```
///
/// ## Fields
///
/// - `data`: The top level crate information and documentation
/// - `included`: The rest of the crate information and documentation
#[derive(Serialize, Debug)]
pub struct Documentation<'a> {
    data: Option<Document<'a>>,
    included: Option<Vec<Document<'a>>>,
}

/// A sub type of the `Documentation` struct. It contains the majority of the data. It can be used
/// for both the `data` and `included` field in the serialized JSON.
///
/// ## Fields
///
/// - `ty`: The type of the the item (e.g. "crate", "function", "enum", etc.)
/// - `id`: The unique identifier associated with this item
/// - `attributes`: The attributes associated with the item like documentation or it's name
/// - `relationships`: An optional field used to show the relationship between the crate to the
///   otheritems in the crate
#[derive(Serialize, Debug)]
pub struct Document<'a> {
    #[serde(rename = "type")]
    ty: &'a str,
    id: &'a str,
    attributes: HashMap<&'a str, &'a str>,
    relationships: Option<HashMap<&'a str, HashMap<&'a str, Vec<Data<'a>>>>>,
}

/// Used to populate the `relationships` `data` field in the serialized JSON
///
/// ## Fields
///
/// - `ty`: The type of the the item (e.g. "crate", "function", "enum", etc.)
/// - `id`: The unique identifier associated with this item
#[derive(Serialize, Debug)]
pub struct Data<'a> {
    #[serde(rename = "type")]
    ty: &'a str,
    id: &'a str,
}

impl<'a> Documentation<'a> {
    /// Create an empty `Documentation` struct
    pub fn new() -> Self {
        Self {
            data: None,
            included: None,
        }
    }

    /// Set the `data` field of a `Documentation` struct to the value passed into the
    /// `data` argument
    pub fn data(mut self, data: Document<'a>) -> Self {
        self.data = Some(data);
        self
    }

    /// Set the `included` field of a `Documentation` struct to the value passed into the
    /// `included` argument
    pub fn included(mut self, included: Vec<Document<'a>>) -> Self {
        self.included = Some(included);
        self
    }
}

impl<'a> Document<'a> {
    /// Create an empty `Document` struct
    pub fn new() -> Self {
        Self {
            ty: "",
            id: "",
            attributes: HashMap::new(),
            relationships: None,
        }
    }

    /// Set the `ty` field of a `Document` struct to the value passed into the
    /// `t` argument
    pub fn ty(mut self, t: &'a str) -> Self {
        self.ty = t;
        self
    }

    /// Set the `id` field of a `Document` struct to the value passed into the
    /// `id` argument
    pub fn id(mut self, id: &'a str) -> Self {
        self.id = id;
        self
    }

    /// Insert an attribute for the `attribute` field of a `Document` struct. If the current
    /// `attribute` exists it'll be overwritten with the given value, otherwise it'll just be
    /// created for the first time.
    pub fn attributes(mut self, attribute: &'a str, value: &'a str) -> Self {
        self.attributes.insert(attribute, value);
        self
    }

    /// Insert a relationship for the `relationships` field of a `Document` struct. If the current
    /// `ty` exists it'll be overwritten with the given `data` value, otherwise it'll just be
    /// created for the first time.
    pub fn relationships(&mut self, ty: &'a str, data: Vec<Data<'a>>) {
        match self.relationships {
            // The code below allows us to handle with ease the fact we have data structured like:
            //
            // "relationships": {
            //     "modules": {
            //         "data": [
            //             {
            //                 "type": "module",
            //                 "id": "example::example_module"
            //             }
            //         ]
            //     }
            //     "type": {
            //         "data": [
            //             {
            //                 "type": "type",
            //                 "id": "example::example_type"
            //             }
            //         ]
            //     }
            // }
            //
            // Where they all have the data field because of JSON API. However, having others
            // handle this with the builder pattern having three fields can be cumbersome. Unless
            // more fields like "data" are added this method handles it automatically.

            // If the `relationships` field does not exist, add the new type relationship (e.g.
            // module, fn, type, etc.) and the data to a newly created relationship map and assign
            // it to the `Document`.
            None => {
                let mut data_map = HashMap::with_capacity(DATA_SIZE);
                data_map.insert("data", data);

                let mut relationships = HashMap::with_capacity(METADATA_SIZE);
                relationships.insert(ty, data_map);
                self.relationships = Some(relationships);
            }
            // If the `relationships` field exists, add the new type relationship (e.g. module, fn,
            // type, etc.) and the data.
            Some(ref mut relationships) => {
                let mut data_map = HashMap::with_capacity(DATA_SIZE);
                data_map.insert("data", data);

                relationships.insert(ty, data_map);
            }
        }
    }
}

impl<'a> Data<'a> {
    /// Create a new empty `Data` struct
    pub fn new() -> Self {
        Self { ty: "", id: "" }
    }

    /// Set the `ty` field of `Data` to the value passed to the `t` argument
    pub fn ty(mut self, t: &'a str) -> Self {
        self.ty = t;
        self
    }

    /// Set the `id` field of `Data` to the value passed to the `id` argument
    pub fn id(mut self, id: &'a str) -> Self {
        self.id = id;
        self
    }
}
