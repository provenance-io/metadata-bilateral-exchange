use cosmwasm_std::Response;

pub fn single_attribute_for_key<'a, T>(response: &'a Response<T>, key: &'a str) -> &'a str {
    response
        .attributes
        .iter()
        .find(|attr| attr.key.as_str() == key)
        .unwrap_or_else(|| panic!("expected to find an attribute with key [{}]", key))
        .value
        .as_str()
}
