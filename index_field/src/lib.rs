pub trait IndexField {
    fn get_field(&self, path: &str) -> Result<serde_json::Value, String>;
    fn set_field(&mut self, path: &str, value: serde_json::Value) -> Result<(), String>;
}

pub fn get_field<T: IndexField>(value: &T, path: &str) -> Result<serde_json::Value, String> {
    value.get_field(path)
}

pub fn set_field<T: IndexField>(
    value: &mut T,
    path: &str,
    new_value: serde_json::Value,
) -> Result<(), String> {
    value.set_field(path, new_value)
}
