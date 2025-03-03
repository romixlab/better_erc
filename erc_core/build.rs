fn main() {
    println!("cargo:rerun-if-changed=../test_schematics/sources");
    generate_netlists::generate_netlists();
}
