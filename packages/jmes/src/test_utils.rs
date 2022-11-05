use cw_multi_test::AppResponse;

pub fn get_attribute(res: &AppResponse, attr: &str) -> String {
    res.events
        .iter()
        .find(|e| e.ty == "wasm")
        .unwrap()
        .attributes
        .iter()
        .find(|a| a.key == attr)
        .unwrap()
        .value
        .clone()
}
