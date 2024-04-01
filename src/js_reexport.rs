use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_name = cashAssemblyToBin, js_namespace = ["window", "reexports"])]
    fn impl_cash_assembly_to_bin(script: &str) -> Result<Box<[u8]>, JsValue>;

    #[wasm_bindgen(js_name = disassembleBytecodeBCH, js_namespace = ["window", "reexports"])]
    pub fn bin_to_cash_assembly(bytecode: Box<[u8]>) -> String;
}

pub fn cash_assembly_to_bin(script: &str) -> Result<Box<[u8]>, anyhow::Error> {
    impl_cash_assembly_to_bin(script).map_err(|e| anyhow::anyhow!(e.as_string().unwrap()))
}
