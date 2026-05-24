//! Walk [`crate::BUNDLED`] and return one typed body triple per
//! `*.yaml`. Order is deterministic per `include_dir` directory order.

use std::sync::Arc;

use starter_flow::definition::body::FlowBody;
use starter_flow_spi::flow::{FlowId, FlowRevisionId};

use crate::convert::convert;
use crate::error::LoadError;
use crate::yaml::parse_yaml;
use crate::BUNDLED;

/// Walk [`crate::BUNDLED`] and convert every `*.yaml` file.
pub fn load_all() -> Result<Vec<(FlowId, FlowRevisionId, FlowBody)>, LoadError> {
    let mut out = Vec::new();
    walk(&BUNDLED, &mut out)?;
    Ok(out)
}

fn walk(
    dir: &include_dir::Dir<'_>,
    out: &mut Vec<(FlowId, FlowRevisionId, FlowBody)>,
) -> Result<(), LoadError> {
    for entry in dir.entries() {
        match entry {
            include_dir::DirEntry::File(f) => {
                let path = f.path().to_string_lossy().into_owned();
                if !(path.ends_with(".yaml") || path.ends_with(".yml")) {
                    continue;
                }
                let yaml = parse_yaml(&path, f.contents())?;
                out.push(convert(&path, yaml)?);
            }
            include_dir::DirEntry::Dir(sub) => walk(sub, out)?,
        }
    }
    Ok(())
}

/// Convert a slice of triples into shared-ownership form.
pub fn into_arcs(
    triples: Vec<(FlowId, FlowRevisionId, FlowBody)>,
) -> Vec<(FlowId, FlowRevisionId, Arc<FlowBody>)> {
    triples
        .into_iter()
        .map(|(id, rev, body)| (id, rev, Arc::new(body)))
        .collect()
}
