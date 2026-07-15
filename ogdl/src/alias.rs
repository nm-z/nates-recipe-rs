use super::{Graph, LazyLock};

pub static OGDL: LazyLock<Graph> = LazyLock::new(Graph::empty);
