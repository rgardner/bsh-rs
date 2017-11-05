// This file comes directly from the lalrpop documentation
// https://github.com/nikomatsakis/lalrpop/blob/master/doc/tutorial.md
extern crate lalrpop;

fn main() {
    lalrpop::process_root().expect("failed to process lalrpop files");
}
