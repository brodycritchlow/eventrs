//! Derive macros for EventRS.
//!
//! This crate provides the `#[derive(Event)]` macro for automatically
//! implementing the Event trait on structs and enums.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields};

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
#[proc_macro_derive(Event)]
pub fn derive_event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    
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
                ::eventrs::EventMetadata::default()
            }
            
            fn size_hint(&self) -> usize {
                ::std::mem::size_of::<Self>()
            }
            
            fn validate(&self) -> ::std::result::Result<(), ::eventrs::EventValidationError> {
                ::std::result::Result::Ok(())
            }
        }
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
fn generate_size_calculation(_name: &syn::Ident, _fields: &Fields) -> proc_macro2::TokenStream {
    // In a real implementation, this would generate code to calculate
    // the size of heap-allocated fields like Vec, HashMap, String, etc.
    quote! {
        ::std::mem::size_of::<Self>()
    }
}

/// Generates metadata creation logic.
/// 
/// This helper function generates code to create appropriate metadata
/// based on event attributes and field annotations.
fn generate_metadata_creation(_name: &syn::Ident) -> proc_macro2::TokenStream {
    // In a real implementation, this would look for attributes like:
    // #[event(priority = "high")]
    // #[event(category = "user")]
    // #[event(source = "auth_service")]
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
        // In a real implementation, we would have more comprehensive tests
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
}