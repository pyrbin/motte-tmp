extern crate embed_resource;
use std::env;

fn main() {
    if env::var("TARGET").unwrap().contains("windows") {
        embed_resource::compile("../../build/win/icon.rc", embed_resource::NONE);
    }
}
