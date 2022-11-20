use cw_multi_test::AppResponse;

pub fn get_attribute(res: &AppResponse, event: &str, attr: &str) -> String {
    res.events
        .iter()
        .find(|e| e.ty == event)
        .unwrap()
        .attributes
        .iter()
        .find(|a| a.key == attr)
        .unwrap()
        .value
        .clone()
}
