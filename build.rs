fn main() {
    pyo3_build_config::use_pyo3_cfgs();

    if std::env::var_os("CARGO_FEATURE_PYTHON").is_some() {
        pyo3_build_config::add_extension_module_link_args();
    }
}
