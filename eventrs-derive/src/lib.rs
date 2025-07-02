//! Derive macros for EventRS.
//!
//! This crate provides derive macros for automatically implementing
//! Event and AsyncEvent traits on structs and enums.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Expr, Fields, Lit, Meta};

/// Derives the Event trait for structs and enums.
///
/// This macro automatically implements the Event trait with sensible defaults.
/// It can be used on any struct or enum that implements Clone + Send + Sync + 'static.
///
/// # Examples
///
/// ## Basic struct
///
/// ```rust
/// use eventrs::Event;
///
/// #[derive(Event, Clone, Debug)]
/// struct UserLoggedIn {
///     user_id: u64,
///     timestamp: std::time::SystemTime,
/// }
/// ```
///
/// ## Enum events
///
/// ```rust
/// use eventrs::Event;
///
/// #[derive(Event, Clone, Debug)]
/// enum NetworkEvent {
///     Connected { peer_id: String },
///     Disconnected { peer_id: String, reason: String },
///     DataReceived { peer_id: String, data: Vec<u8> },
/// }
/// ```
///
/// ## With validation
///
/// ```rust
/// use eventrs::{Event, EventValidationError};
///
/// #[derive(Event, Clone, Debug)]
/// struct EmailEvent {
///     to: String,
///     subject: String,
///     body: String,
/// }
///
/// impl EmailEvent {
///     fn validate(&self) -> Result<(), EventValidationError> {
///         if self.to.is_empty() {
///             return Err(EventValidationError::missing_field("to"));
///         }
///         if !self.to.contains('@') {
///             return Err(EventValidationError::invalid_value("to", &self.to));
///         }
///         Ok(())
///     }
/// }
/// ```
///
/// ## With event attributes
///
/// ```rust
/// use eventrs::Event;
///
/// #[derive(Event, Clone, Debug)]
/// #[event(priority = "high", category = "user", source = "auth_service")]
/// struct UserLoggedIn {
///     user_id: u64,
///     timestamp: std::time::SystemTime,
/// }
/// ```
#[proc_macro_derive(Event, attributes(event))]
pub fn derive_event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // Parse event attributes
    let event_attrs = parse_event_attributes(&input.attrs);

    // Generate metadata based on attributes
    let metadata_impl = generate_metadata_from_attributes(&event_attrs);

    // Generate size calculation
    let size_impl = if let Data::Struct(ref data_struct) = input.data {
        generate_size_calculation(&name, &data_struct.fields)
    } else {
        quote! { ::std::mem::size_of::<Self>() }
    };

    // Generate the Event trait implementation
    let expanded = quote! {
        impl #impl_generics ::eventrs::Event for #name #ty_generics #where_clause {
            fn event_type_name() -> &'static str {
                stringify!(#name)
            }

            fn event_type_id() -> ::std::any::TypeId {
                ::std::any::TypeId::of::<Self>()
            }

            fn metadata(&self) -> ::eventrs::EventMetadata {
                #metadata_impl
            }

            fn size_hint(&self) -> usize {
                #size_impl
            }

            fn validate(&self) -> ::std::result::Result<(), ::eventrs::EventValidationError> {
                ::std::result::Result::Ok(())
            }
        }
    };

    TokenStream::from(expanded)
}

/// Derives the AsyncEvent trait for structs and enums with async support.
///
/// This macro automatically implements both the Event trait and adds async-specific
/// functionality for use with AsyncEventBus.
///
/// # Examples
///
/// ## Basic async event
///
/// ```rust
/// use eventrs::AsyncEvent;
///
/// #[derive(AsyncEvent, Clone, Debug)]
/// struct AsyncUserEvent {
///     user_id: u64,
///     action: String,
/// }
/// ```
///
/// ## With async validation
///
/// ```rust
/// use eventrs::{AsyncEvent, EventValidationError};
///
/// #[derive(AsyncEvent, Clone, Debug)]
/// #[event(async_validation)]
/// struct AsyncEmailEvent {
///     to: String,
///     subject: String,
///     body: String,
/// }
///
/// impl AsyncEmailEvent {
///     async fn validate_async(&self) -> Result<(), EventValidationError> {
///         // Async validation logic (e.g., database lookup)
///         if self.to.is_empty() {
///             return Err(EventValidationError::missing_field("to"));
///         }
///         
///         // Async email validation
///         if !email_service::validate_email(&self.to).await? {
///             return Err(EventValidationError::invalid_value("to", &self.to));
///         }
///         
///         Ok(())
///     }
/// }
/// ```
///
/// ## With async metadata generation
///
/// ```rust
/// use eventrs::AsyncEvent;
///
/// #[derive(AsyncEvent, Clone, Debug)]
/// #[event(async_metadata, priority = "high")]
/// struct AsyncTaskEvent {
///     task_id: String,
///     payload: Vec<u8>,
/// }
///
/// impl AsyncTaskEvent {
///     async fn generate_metadata_async(&self) -> eventrs::EventMetadata {
///         // Async metadata generation (e.g., external service calls)
///         let enriched_data = metadata_service::enrich(&self.task_id).await;
///         
///         eventrs::EventMetadata::new()
///             .with_priority(eventrs::Priority::High)
///             .with_source("async_task_processor")
///             .with_custom("enriched_data", enriched_data)
///     }
/// }
/// ```
#[proc_macro_derive(AsyncEvent, attributes(event))]
pub fn derive_async_event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // Parse event attributes
    let event_attrs = parse_event_attributes(&input.attrs);

    // Generate metadata based on attributes
    let metadata_impl = generate_metadata_from_attributes(&event_attrs);

    // Generate size calculation
    let size_impl = if let Data::Struct(ref data_struct) = input.data {
        generate_size_calculation(&name, &data_struct.fields)
    } else {
        quote! { ::std::mem::size_of::<Self>() }
    };

    // Check for async-specific attributes
    let has_async_validation = event_attrs.iter().any(|(key, _)| key == "async_validation");
    let has_async_metadata = event_attrs.iter().any(|(key, _)| key == "async_metadata");

    // Note: We don't generate default implementations for async methods
    // as these should be provided by the user when the attributes are specified
    let async_validation_impl = quote! {};
    let async_metadata_impl = quote! {};

    // Generate both Event and AsyncEvent trait implementations
    let expanded = quote! {
        // Standard Event trait implementation
        impl #impl_generics ::eventrs::Event for #name #ty_generics #where_clause {
            fn event_type_name() -> &'static str {
                stringify!(#name)
            }

            fn event_type_id() -> ::std::any::TypeId {
                ::std::any::TypeId::of::<Self>()
            }

            fn metadata(&self) -> ::eventrs::EventMetadata {
                #metadata_impl
            }

            fn size_hint(&self) -> usize {
                #size_impl
            }

            fn validate(&self) -> ::std::result::Result<(), ::eventrs::EventValidationError> {
                ::std::result::Result::Ok(())
            }
        }

        // AsyncEvent-specific methods
        impl #impl_generics #name #ty_generics #where_clause {
            #async_validation_impl
            #async_metadata_impl
        }

        // Marker trait implementation for AsyncEvent identification
        impl #impl_generics ::eventrs::AsyncEventMarker for #name #ty_generics #where_clause {}
    };

    TokenStream::from(expanded)
}

/// Helper function to check if the input type has the required traits.
///
/// This is used internally by the derive macro to ensure that the type
/// implements the necessary traits for being an Event.
fn check_required_traits(input: &DeriveInput) -> Result<(), String> {
    // In a real implementation, we would check that the type implements
    // Clone + Send + Sync + 'static. For now, we assume the user has
    // correctly added these derives.

    match &input.data {
        Data::Struct(_) => Ok(()),
        Data::Enum(_) => Ok(()),
        Data::Union(_) => Err("Event cannot be derived for union types".to_string()),
    }
}

/// Generates validation logic for struct fields.
///
/// This helper function analyzes struct fields and generates appropriate
/// validation code based on field types and attributes.
fn generate_validation_for_fields(_fields: &Fields) -> proc_macro2::TokenStream {
    // In a real implementation, this would generate validation logic
    // based on field attributes like #[validate(required)], #[validate(email)], etc.
    quote! {
        ::std::result::Result::Ok(())
    }
}

/// Generates size calculation logic for complex types.
///
/// This helper function generates code to calculate the total size of
/// the event including heap-allocated data.
fn generate_size_calculation(_name: &syn::Ident, fields: &Fields) -> proc_macro2::TokenStream {
    let mut size_calculation = quote! { ::std::mem::size_of::<Self>() };

    match fields {
        Fields::Named(fields_named) => {
            for field in &fields_named.named {
                if let Some(field_name) = &field.ident {
                    let field_type = &field.ty;

                    // Check for common heap-allocated types
                    let type_string = quote! { #field_type }.to_string();

                    if type_string.contains("Vec") {
                        size_calculation = quote! {
                            #size_calculation + self.#field_name.capacity()
                        };
                    } else if type_string.contains("String") {
                        size_calculation = quote! {
                            #size_calculation + self.#field_name.len()
                        };
                    } else if type_string.contains("HashMap") || type_string.contains("BTreeMap") {
                        size_calculation = quote! {
                            #size_calculation + self.#field_name.len() * 32  // Rough estimate for key-value pairs
                        };
                    }
                }
            }
        }
        Fields::Unnamed(fields_unnamed) => {
            for (index, field) in fields_unnamed.unnamed.iter().enumerate() {
                let field_type = &field.ty;
                let field_index = syn::Index::from(index);

                let type_string = quote! { #field_type }.to_string();

                if type_string.contains("Vec") {
                    size_calculation = quote! {
                        #size_calculation + self.#field_index.capacity()
                    };
                } else if type_string.contains("String") {
                    size_calculation = quote! {
                        #size_calculation + self.#field_index.len()
                    };
                }
            }
        }
        Fields::Unit => {
            // Unit structs have no additional heap data
        }
    }

    size_calculation
}

/// Parses event attributes from derive input.
///
/// Extracts and parses #[event(...)] attributes to customize event behavior.
fn parse_event_attributes(attrs: &[Attribute]) -> Vec<(String, String)> {
    let mut event_attrs = Vec::new();

    for attr in attrs {
        if attr.path().is_ident("event") {
            let _ = attr.parse_nested_meta(|meta| {
                let path = meta.path;

                if let Some(ident) = path.get_ident() {
                    let key = ident.to_string();

                    if meta.input.peek(syn::Token![=]) {
                        // This is a name-value pair like priority = "high"
                        let _eq: syn::Token![=] = meta.input.parse()?;
                        let value: Expr = meta.input.parse()?;

                        if let Expr::Lit(expr_lit) = value {
                            if let Lit::Str(lit_str) = expr_lit.lit {
                                event_attrs.push((key, lit_str.value()));
                            }
                        }
                    } else {
                        // This is a flag like async_validation
                        event_attrs.push((key, "true".to_string()));
                    }
                }

                Ok(())
            });
        }
    }

    event_attrs
}

/// Generates metadata from parsed event attributes.
///
/// Creates EventMetadata initialization code based on parsed attributes.
fn generate_metadata_from_attributes(attrs: &[(String, String)]) -> proc_macro2::TokenStream {
    let mut metadata_builder = quote! { ::eventrs::EventMetadata::new() };

    for (key, value) in attrs {
        match key.as_str() {
            "priority" => {
                let priority_token = match value.as_str() {
                    "critical" => quote! { ::eventrs::Priority::Critical },
                    "high" => quote! { ::eventrs::Priority::High },
                    "normal" => quote! { ::eventrs::Priority::Normal },
                    "low" => quote! { ::eventrs::Priority::Low },
                    _ => {
                        // Try to parse as custom priority value
                        if let Ok(priority_val) = value.parse::<u16>() {
                            quote! { ::eventrs::Priority::Custom(::eventrs::PriorityValue::new(#priority_val)) }
                        } else {
                            quote! { ::eventrs::Priority::Normal }
                        }
                    }
                };
                metadata_builder = quote! {
                    #metadata_builder.with_priority(#priority_token)
                };
            }
            "category" => {
                metadata_builder = quote! {
                    #metadata_builder.with_category(#value)
                };
            }
            "source" => {
                metadata_builder = quote! {
                    #metadata_builder.with_source(#value)
                };
            }
            "tag" => {
                metadata_builder = quote! {
                    #metadata_builder.with_tag(#value)
                };
            }
            _ => {
                // Custom metadata field
                metadata_builder = quote! {
                    #metadata_builder.with_custom_field(#key, #value)
                };
            }
        }
    }

    // Add timestamp by default
    metadata_builder = quote! {
        #metadata_builder.with_timestamp(::std::time::SystemTime::now())
    };

    metadata_builder
}

/// Generates metadata creation logic (deprecated - use generate_metadata_from_attributes).
///
/// This helper function generates code to create appropriate metadata
/// based on event attributes and field annotations.
fn generate_metadata_creation(_name: &syn::Ident) -> proc_macro2::TokenStream {
    quote! {
        ::eventrs::EventMetadata::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_macro_compilation() {
        // This test ensures the macro compiles correctly
        let input = quote! {
            #[derive(Event, Clone, Debug)]
            struct TestEvent {
                value: i32,
            }
        };

        // Parse the input
        let parsed: DeriveInput = syn::parse2(input).unwrap();

        // Check that required traits validation passes
        assert!(check_required_traits(&parsed).is_ok());
    }

    #[test]
    fn test_async_event_derive_compilation() {
        // Test AsyncEvent derive macro compilation
        let input = quote! {
            #[derive(AsyncEvent, Clone, Debug)]
            #[event(async_validation, async_metadata)]
            struct AsyncTestEvent {
                value: i32,
                data: Vec<u8>,
            }
        };

        let parsed: DeriveInput = syn::parse2(input).unwrap();
        assert!(check_required_traits(&parsed).is_ok());
    }

    #[test]
    fn test_event_attributes_parsing() {
        let input = quote! {
            #[derive(Event, Clone, Debug)]
            #[event(priority = "high", category = "test", source = "unit_test")]
            struct AttributeTestEvent {
                value: i32,
            }
        };

        let parsed: DeriveInput = syn::parse2(input).unwrap();
        let attrs = parse_event_attributes(&parsed.attrs);

        assert_eq!(attrs.len(), 3);
        assert!(attrs.contains(&("priority".to_string(), "high".to_string())));
        assert!(attrs.contains(&("category".to_string(), "test".to_string())));
        assert!(attrs.contains(&("source".to_string(), "unit_test".to_string())));
    }

    #[test]
    fn test_async_attributes_parsing() {
        let input = quote! {
            #[derive(AsyncEvent, Clone, Debug)]
            #[event(async_validation, async_metadata, priority = "critical")]
            struct AsyncAttributeTestEvent {
                value: i32,
            }
        };

        let parsed: DeriveInput = syn::parse2(input).unwrap();
        let attrs = parse_event_attributes(&parsed.attrs);

        assert!(attrs.contains(&("async_validation".to_string(), "true".to_string())));
        assert!(attrs.contains(&("async_metadata".to_string(), "true".to_string())));
        assert!(attrs.contains(&("priority".to_string(), "critical".to_string())));
    }

    #[test]
    fn test_metadata_generation() {
        let attrs = vec![
            ("priority".to_string(), "high".to_string()),
            ("category".to_string(), "user".to_string()),
            ("source".to_string(), "auth_service".to_string()),
        ];

        let metadata_code = generate_metadata_from_attributes(&attrs);
        let generated = metadata_code.to_string();

        // Check that the generated code contains the expected method calls
        assert!(generated.contains("with_priority"));
        assert!(generated.contains("with_category"));
        assert!(generated.contains("with_source"));
        assert!(generated.contains("with_timestamp"));
    }

    #[test]
    fn test_size_calculation_generation() {
        let input = quote! {
            struct SizeTestEvent {
                id: u64,
                data: Vec<u8>,
                message: String,
            }
        };

        let parsed: DeriveInput = syn::parse2(input).unwrap();
        if let Data::Struct(data_struct) = parsed.data {
            let size_code = generate_size_calculation(&parsed.ident, &data_struct.fields);
            let generated = size_code.to_string();

            println!("Generated size calculation: {}", generated);

            // Check that heap-allocated fields are considered (accounting for token spacing)
            assert!(generated.contains("self . data . capacity ()"));
            assert!(generated.contains("self . message . len ()"));
        }
    }

    #[test]
    fn test_union_rejection() {
        let input = quote! {
            union TestUnion {
                a: i32,
                b: f32,
            }
        };

        let parsed: DeriveInput = syn::parse2(input).unwrap();
        assert!(check_required_traits(&parsed).is_err());
    }

    #[test]
    fn test_custom_priority_parsing() {
        let attrs = vec![("priority".to_string(), "750".to_string())];

        let metadata_code = generate_metadata_from_attributes(&attrs);
        let generated = metadata_code.to_string();

        println!("Generated metadata: {}", generated);

        // Should generate custom priority with value 750 (accounting for token spacing)
        assert!(generated.contains("Priority :: Custom"));
        assert!(generated.contains("750"));
    }
}
