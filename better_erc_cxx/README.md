# better_erc_cxx

better_erc for C++ hosts. A cxx bridge with one call: `berc::check(project, edited)` opens the design file `project`, replaces the files named in `edited` with the texts given (an editor's unsaved sheets), runs every check and returns the violations as plain structs. Errors and panics come back as a C++ exception (`rust::Error`).

A KiCad fork builds it with Corrosion (`corrosion_import_crate` and `corrosion_add_cxxbridge`) and links the static library into eeschema with `--whole-archive`, so that every check registered through `inventory` stays in the link.
