extern crate adblock;

use adblock::engine::Engine;
use adblock::filter_lists;
use adblock::resources::{Resource, ResourceType, MimeType};
use core::ptr;
use libc::size_t;
use std::ffi::CStr;
use std::ffi::CString;
use std::os::raw::c_char;
use std::string::String;

#[repr(C)]
pub struct FList {
    uuid: *const c_char,
    url: *const c_char,
    title: *const c_char,
    lang: *const c_char,
    lang2: *const c_char,
    lang3: *const c_char,
    support_url: *const c_char,
    component_id: *const c_char,
    base64_public_key: *const c_char,
    desc: *const c_char,
}

/// Create a new `Engine`.
#[no_mangle]
pub unsafe extern "C" fn engine_create(rules: *const c_char) -> *mut Engine {
    let rules = CStr::from_ptr(rules).to_str().unwrap();
    let mut filter_set = adblock::lists::FilterSet::new(false);
    filter_set.add_filter_list(&rules, adblock::lists::FilterFormat::Standard);
    let engine = Engine::from_filter_set(filter_set, true);
    Box::into_raw(Box::new(engine))
}

/// Checks if a `url` matches for the specified `Engine` within the context.
#[no_mangle]
pub unsafe extern "C" fn engine_match(
    engine: *mut Engine,
    url: *const c_char,
    host: *const c_char,
    tab_host: *const c_char,
    third_party: bool,
    resource_type: *const c_char,
    explicit_cancel: *mut bool,
    saved_from_exception: *mut bool,
    redirect: *mut *mut c_char,
) -> bool {
    let url = CStr::from_ptr(url).to_str().unwrap();
    let host = CStr::from_ptr(host).to_str().unwrap();
    let tab_host = CStr::from_ptr(tab_host).to_str().unwrap();
    let resource_type = CStr::from_ptr(resource_type).to_str().unwrap();
    assert!(!engine.is_null());
    let engine = Box::leak(Box::from_raw(engine));
    let blocker_result = engine.check_network_urls_with_hostnames(
        url,
        host,
        tab_host,
        resource_type,
        Some(third_party),
    );
    *explicit_cancel = blocker_result.explicit_cancel;
    *saved_from_exception = blocker_result.filter != None && blocker_result.exception != None;
    *redirect = match blocker_result.redirect {
        Some(x) => match CString::new(x) {
            Ok(y) => y.into_raw(),
            _ => ptr::null_mut(),
        },
        None => ptr::null_mut(),
    };
    blocker_result.matched
}

/// Adds a tag to the engine for consideration
#[no_mangle]
pub unsafe extern "C" fn engine_add_tag(engine: *mut Engine, tag: *const c_char) {
    let tag = CStr::from_ptr(tag).to_str().unwrap();
    assert!(!engine.is_null());
    let engine = Box::leak(Box::from_raw(engine));
    engine.enable_tags(&[tag]);
}

/// Checks if a tag exists in the engine
#[no_mangle]
pub unsafe extern "C" fn engine_tag_exists(engine: *mut Engine, tag: *const c_char) -> bool {
    let tag = CStr::from_ptr(tag).to_str().unwrap();
    assert!(!engine.is_null());
    let engine = Box::leak(Box::from_raw(engine));
    engine.tag_exists(tag)
}

/// Adds a resource to the engine by name
#[no_mangle]
pub unsafe extern "C" fn engine_add_resource(
    engine: *mut Engine,
    key: *const c_char,
    content_type: *const c_char,
    data: *const c_char,
) {
    let key = CStr::from_ptr(key).to_str().unwrap();
    let content_type = CStr::from_ptr(content_type).to_str().unwrap();
    let data = CStr::from_ptr(data).to_str().unwrap();
    let resource = Resource {
        name: key.to_string(),
        aliases: vec![],
        kind: ResourceType::Mime(MimeType::from(std::borrow::Cow::from(content_type))),
        content: data.to_string(),
    };
    assert!(!engine.is_null());
    let engine = Box::leak(Box::from_raw(engine));
    engine.add_resource(resource);
}

/// Adds a list of `Resource`s from JSON format
#[no_mangle]
pub unsafe extern "C" fn engine_add_resources(engine: *mut Engine, resources: *const c_char) {
    let resources = CStr::from_ptr(resources).to_str().unwrap();
    let resources: Vec<Resource> = serde_json::from_str(resources).unwrap_or_else(|e| {
        eprintln!("Failed to parse JSON adblock resources: {}", e);
        vec![]
    });
    assert!(!engine.is_null());
    let engine = Box::leak(Box::from_raw(engine));
    engine.use_resources(&resources);
}

/// Removes a tag to the engine for consideration
#[no_mangle]
pub unsafe extern "C" fn engine_remove_tag(engine: *mut Engine, tag: *const c_char) {
    let tag = CStr::from_ptr(tag).to_str().unwrap();
    assert!(!engine.is_null());
    let engine = Box::leak(Box::from_raw(engine));
    engine.disable_tags(&[tag]);
}

/// Deserializes a previously serialized data file list.
#[no_mangle]
pub unsafe extern "C" fn engine_deserialize(
    engine: *mut Engine,
    data: *const c_char,
    data_size: size_t,
) -> bool {
    let data: &[u8] = std::slice::from_raw_parts(data as *const u8, data_size);
    assert!(!engine.is_null());
    let engine = Box::leak(Box::from_raw(engine));
    engine.deserialize(&data).is_ok()
}

/// Destroy a `Engine` once you are done with it.
#[no_mangle]
pub unsafe extern "C" fn engine_destroy(engine: *mut Engine) {
    if !engine.is_null() {
        drop(Box::from_raw(engine));
    }
}

/// Destroy a `*c_char` once you are done with it.
#[no_mangle]
pub unsafe extern "C" fn c_char_buffer_destroy(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

/// Get the default list size. `category` must be one of "regions" or "default"
#[no_mangle]
pub unsafe extern "C" fn filter_list_size(category: *const c_char) -> size_t {
    if CStr::from_ptr(category).to_str().unwrap() == "regions" {
        filter_lists::regions::regions().len()
    } else {
        filter_lists::default::default_lists().len()
    }
}

/// Get the specific default list size
#[no_mangle]
pub unsafe extern "C" fn filter_list_get(category: *const c_char, i: size_t) -> FList {
    let list = match CStr::from_ptr(category).to_str().unwrap() {
        "regions" => filter_lists::regions::regions()[i].clone(),
        _ => filter_lists::default::default_lists()[i].clone(),
    };
    let mut new_list = FList {
        uuid: CString::new(list.uuid)
            .expect("Error: CString::new()")
            .into_raw(),
        url: CString::new(list.url)
            .expect("Error: CString::new()")
            .into_raw(),
        title: CString::new(list.title)
            .expect("Error: CString::new()")
            .into_raw(),
        lang: CString::new("").expect("Error: CString::new()").into_raw(),
        lang2: CString::new("").expect("Error: CString::new()").into_raw(),
        lang3: CString::new("").expect("Error: CString::new()").into_raw(),
        support_url: CString::new(list.support_url)
            .expect("Error: CString::new()")
            .into_raw(),
        component_id: CString::new(list.component_id)
            .expect("Error: CString::new()")
            .into_raw(),
        base64_public_key: CString::new(list.base64_public_key)
            .expect("Error: CString::new()")
            .into_raw(),
        desc: CString::new(list.desc)
            .expect("Error: CString::new()")
            .into_raw(),
    };
    if !list.langs.is_empty() {
        new_list.lang = CString::new(list.langs[0].clone())
            .expect("Error: CString::new()")
            .into_raw();
    }
    if list.langs.len() > 1 {
        new_list.lang2 = CString::new(list.langs[1].clone())
            .expect("Error: CString::new()")
            .into_raw();
    }
    if list.langs.len() > 2 {
        new_list.lang3 = CString::new(list.langs[2].clone())
            .expect("Error: CString::new()")
            .into_raw();
    }
    new_list
}

/// Returns a set of cosmetic filtering resources specific to the given url, in JSON format
#[no_mangle]
pub unsafe extern "C" fn engine_url_cosmetic_resources(
    engine: *mut Engine,
    url: *const c_char,
) -> *mut c_char {
    let url = CStr::from_ptr(url).to_str().unwrap();
    assert!(!engine.is_null());
    let engine = Box::leak(Box::from_raw(engine));
    let ptr = CString::new(serde_json::to_string(&engine.url_cosmetic_resources(url))
        .unwrap_or_else(|_| "".into()))
        .expect("Error: CString::new()")
        .into_raw();
    std::mem::forget(ptr);
    ptr
}

/// Returns a stylesheet containing all generic cosmetic rules that begin with any of the provided class and id selectors
///
/// The leading '.' or '#' character should not be provided
#[no_mangle]
pub unsafe extern "C" fn engine_hidden_class_id_selectors(
    engine: *mut Engine,
    classes: *const *const c_char,
    classes_size: size_t,
    ids: *const *const c_char,
    ids_size: size_t,
    exceptions: *const *const c_char,
    exceptions_size: size_t,
) -> *mut c_char {
    let classes = std::slice::from_raw_parts(classes, classes_size);
    let classes: Vec<String> = (0..classes_size)
        .map(|index| CStr::from_ptr(classes[index]).to_str().unwrap().to_owned())
        .collect();
    let ids = std::slice::from_raw_parts(ids, ids_size);
    let ids: Vec<String> = (0..ids_size)
        .map(|index| CStr::from_ptr(ids[index]).to_str().unwrap().to_owned())
        .collect();
    let exceptions = std::slice::from_raw_parts(exceptions, exceptions_size);
    let exceptions: std::collections::HashSet<String> = (0..exceptions_size)
        .map(|index| CStr::from_ptr(exceptions[index]).to_str().unwrap().to_owned())
        .collect();
    assert!(!engine.is_null());
    let engine = Box::leak(Box::from_raw(engine));
    let stylesheet = engine.hidden_class_id_selectors(&classes, &ids, &exceptions);
    CString::new(serde_json::to_string(&stylesheet).unwrap_or_else(|_| "".into())).expect("Error: CString::new()").into_raw()
}
