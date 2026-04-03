use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Field, Fields, Ident, Type};

/// Determines if a field has the `#[sync(nested)]` attribute.
fn is_nested(field: &Field) -> bool {
    for attr in &field.attrs {
        if attr.path().is_ident("sync") {
            if let Ok(nested) = attr.parse_args::<Ident>() {
                if nested == "nested" {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if a type is `BTreeMap<K, V>` and return (K, V) if so.
fn extract_btreemap_types(ty: &Type) -> Option<(&Type, &Type)> {
    if let Type::Path(type_path) = ty {
        let segment = type_path.path.segments.last()?;
        if segment.ident == "BTreeMap" {
            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                let mut types = args.args.iter().filter_map(|arg| {
                    if let syn::GenericArgument::Type(t) = arg {
                        Some(t)
                    } else {
                        None
                    }
                });
                let k = types.next()?;
                let v = types.next()?;
                return Some((k, v));
            }
        }
    }
    None
}

/// Get the last segment name of a type path.
fn type_name(ty: &Type) -> Option<String> {
    if let Type::Path(type_path) = ty {
        type_path.path.segments.last().map(|s| s.ident.to_string())
    } else {
        None
    }
}

/// For a type name like "OrderLineFull", produce "OrderLinePatch".
/// If the name doesn't end in "Full", just append "Patch".
fn to_patch_name(type_name: &str) -> String {
    if let Some(base) = type_name.strip_suffix("Full") {
        format!("{}Patch", base)
    } else {
        format!("{}Patch", type_name)
    }
}

/// Map a Rust type to its TypeScript equivalent.
fn rust_type_to_ts(ty: &Type) -> String {
    if let Type::Path(type_path) = ty {
        let segment = type_path.path.segments.last().unwrap();
        let name = segment.ident.to_string();
        match name.as_str() {
            "String" => "string".to_string(),
            "bool" => "boolean".to_string(),
            "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f32" | "f64"
            | "isize" | "usize" => "number".to_string(),
            "Decimal" => "string".to_string(),
            "Id" => {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        let tag = type_name(inner).unwrap_or_else(|| "unknown".to_string());
                        return format!("Id<\"{}\">", tag);
                    }
                }
                "Id<\"unknown\">".to_string()
            }
            "BTreeMap" => {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    let mut types = args.args.iter().filter_map(|arg| {
                        if let syn::GenericArgument::Type(t) = arg {
                            Some(t)
                        } else {
                            None
                        }
                    });
                    if let (Some(k), Some(v)) = (types.next(), types.next()) {
                        let k_ts = rust_type_to_ts(k);
                        let v_ts = rust_type_to_ts(v);
                        return format!("Record<{}, {}>", k_ts, v_ts);
                    }
                }
                "Record<string, unknown>".to_string()
            }
            "Option" => {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        let inner_ts = rust_type_to_ts(inner);
                        return format!("{} | null", inner_ts);
                    }
                }
                "unknown | null".to_string()
            }
            // If type already ends in "Full", use as-is (it's a generated Full struct)
            other if other.ends_with("Full") => other.to_string(),
            other => format!("{}Full", other),
        }
    } else {
        "unknown".to_string()
    }
}

/// Map a Rust type to its TS patch equivalent.
fn rust_type_to_ts_patch(ty: &Type, nested: bool) -> String {
    if let Some((_k, v)) = extract_btreemap_types(ty) {
        let k_ts = rust_type_to_ts(_k);
        if nested {
            let v_name = type_name(v).unwrap_or_else(|| "unknown".to_string());
            let patch_name = to_patch_name(&v_name);
            format!("Record<{}, {} | null>", k_ts, patch_name)
        } else {
            let v_ts = rust_type_to_ts(v);
            format!("Record<{}, {} | null>", k_ts, v_ts)
        }
    } else if nested {
        let name = type_name(ty).unwrap_or_else(|| "unknown".to_string());
        format!("{}", to_patch_name(&name))
    } else {
        rust_type_to_ts(ty)
    }
}

#[proc_macro_derive(SyncEntity, attributes(sync))]
pub fn derive_sync_entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let full_name = format_ident!("{}Full", name);
    let patch_name = format_ident!("{}Patch", name);
    let name_str = name.to_string();

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("SyncEntity only supports structs with named fields"),
        },
        _ => panic!("SyncEntity only supports structs"),
    };

    struct FieldInfo<'a> {
        ident: &'a Ident,
        ty: &'a Type,
        nested: bool,
        is_btreemap: bool,
    }

    let field_infos: Vec<FieldInfo> = fields
        .iter()
        .map(|f| {
            let ident = f.ident.as_ref().unwrap();
            let nested = is_nested(f);
            let is_btreemap = extract_btreemap_types(&f.ty).is_some();
            FieldInfo {
                ident,
                ty: &f.ty,
                nested,
                is_btreemap,
            }
        })
        .collect();

    // Full struct fields
    let full_fields = field_infos.iter().map(|fi| {
        let ident = fi.ident;
        let ty = fi.ty;
        quote! { pub #ident: #ty }
    });

    // Patch struct fields
    let patch_fields = field_infos.iter().map(|fi| {
        let ident = fi.ident;
        let ty = fi.ty;

        if fi.is_btreemap {
            let (_k, _v) = extract_btreemap_types(ty).unwrap();
            if fi.nested {
                let v_patch = format_ident!("{}", to_patch_name(&type_name(_v).unwrap()));
                quote! {
                    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
                    pub #ident: BTreeMap<#_k, Option<#v_patch>>
                }
            } else {
                quote! {
                    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
                    pub #ident: BTreeMap<#_k, Option<#_v>>
                }
            }
        } else if fi.nested {
            let v_patch = format_ident!("{}", to_patch_name(&type_name(ty).unwrap()));
            quote! {
                #[serde(default, skip_serializing_if = "Option::is_none")]
                pub #ident: Option<#v_patch>
            }
        } else {
            quote! {
                #[serde(default, skip_serializing_if = "Option::is_none")]
                pub #ident: Option<#ty>
            }
        }
    });

    // Diff fields
    let diff_fields = field_infos.iter().map(|fi| {
        let ident = fi.ident;

        if fi.is_btreemap {
            if fi.nested {
                quote! {
                    let #ident = sync_core::diffable::diff_nested_map(&self.#ident, &other.#ident);
                    if !#ident.is_empty() { has_changes = true; }
                }
            } else {
                quote! {
                    let #ident = sync_core::diffable::diff_leaf_map(&self.#ident, &other.#ident);
                    if !#ident.is_empty() { has_changes = true; }
                }
            }
        } else if fi.nested {
            quote! {
                let #ident = sync_core::Diffable::diff(&self.#ident, &other.#ident);
                if #ident.is_some() { has_changes = true; }
            }
        } else {
            quote! {
                let #ident = sync_core::diffable::diff_leaf(&self.#ident, &other.#ident);
                if #ident.is_some() { has_changes = true; }
            }
        }
    });

    let diff_field_names = field_infos.iter().map(|fi| {
        let ident = fi.ident;
        quote! { #ident }
    });

    // Merge fields
    let merge_fields = field_infos.iter().map(|fi| {
        let ident = fi.ident;

        if fi.is_btreemap {
            if fi.nested {
                quote! {
                    #ident: sync_core::diffable::merge_nested_map(self.#ident, patch.#ident)
                }
            } else {
                quote! {
                    #ident: sync_core::diffable::merge_leaf_map(self.#ident, patch.#ident)
                }
            }
        } else if fi.nested {
            quote! {
                #ident: match patch.#ident {
                    Some(p) => sync_core::Diffable::merge(self.#ident, p),
                    None => self.#ident,
                }
            }
        } else {
            quote! {
                #ident: sync_core::diffable::merge_leaf(self.#ident, patch.#ident)
            }
        }
    });

    // FullToPatch fields
    let to_patch_fields = field_infos.iter().map(|fi| {
        let ident = fi.ident;

        if fi.is_btreemap {
            if fi.nested {
                quote! {
                    #ident: self.#ident.iter().map(|(k, v)| {
                        (k.clone(), Some(sync_core::diffable::FullToPatch::to_patch(v)))
                    }).collect()
                }
            } else {
                quote! {
                    #ident: self.#ident.iter().map(|(k, v)| (k.clone(), Some(v.clone()))).collect()
                }
            }
        } else if fi.nested {
            quote! {
                #ident: Some(sync_core::diffable::FullToPatch::to_patch(&self.#ident))
            }
        } else {
            quote! {
                #ident: Some(self.#ident.clone())
            }
        }
    });

    // TypeScript interface generation
    let ts_full_fields: Vec<String> = field_infos
        .iter()
        .map(|fi| {
            let name = fi.ident.to_string();
            // Convert snake_case to camelCase
            let camel = to_camel_case(&name);
            let ts_type = rust_type_to_ts(fi.ty);
            format!("  readonly {}: {};", camel, ts_type)
        })
        .collect();

    let ts_full_interface = format!(
        "export interface {}Full {{\n{}\n}}\n",
        name_str,
        ts_full_fields.join("\n")
    );

    let ts_patch_fields: Vec<String> = field_infos
        .iter()
        .map(|fi| {
            let name = fi.ident.to_string();
            let camel = to_camel_case(&name);
            let ts_type = rust_type_to_ts_patch(fi.ty, fi.nested);
            format!("  readonly {}?: {};", camel, ts_type)
        })
        .collect();

    let ts_patch_interface = format!(
        "export interface {}Patch {{\n{}\n}}\n",
        name_str,
        ts_patch_fields.join("\n")
    );

    let expanded = quote! {
        #[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct #full_name {
            #(#full_fields,)*
        }

        #[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct #patch_name {
            #(#patch_fields,)*
        }

        impl sync_core::Diffable for #full_name {
            type Patch = #patch_name;

            fn diff(&self, other: &Self) -> Option<Self::Patch> {
                let mut has_changes = false;
                #(#diff_fields)*

                if has_changes {
                    Some(#patch_name {
                        #(#diff_field_names,)*
                    })
                } else {
                    None
                }
            }

            fn merge(self, patch: Self::Patch) -> Self {
                Self {
                    #(#merge_fields,)*
                }
            }
        }

        impl sync_core::diffable::FullToPatch for #full_name {
            type Patch = #patch_name;

            fn to_patch(&self) -> #patch_name {
                #patch_name {
                    #(#to_patch_fields,)*
                }
            }
        }

        inventory::submit! {
            sync_core::TsTypeRegistration {
                entity_name: #name_str,
                ts_full_interface: #ts_full_interface,
                ts_patch_interface: #ts_patch_interface,
            }
        }
    };

    TokenStream::from(expanded)
}

fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;
    for (i, c) in s.chars().enumerate() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(if i == 0 { c.to_ascii_lowercase() } else { c });
        }
    }
    result
}
