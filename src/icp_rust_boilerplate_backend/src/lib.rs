#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_cdk::caller;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

// Struct definition for Saloon
#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Saloon {
    owner: String,
    id: u64,
    name: String,
    location: String,
    saloon_url: String,
    saloonservices: Vec<SaloonService>,
    created_at: u64,
    updated_at: Option<u64>,
}

// Struct definition for Services
#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct SaloonService {
    service_name: String,
    service_description: String,
    created_at: u64,
    updated_at: Option<u64>,
}

// a trait that must be implemented for a struct that is stored in a stable struct
impl Storable for Saloon {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

// a trait that must be implemented for a struct that is stored in a stable struct
impl Storable for SaloonService {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

// another trait that must be implemented for a struct that is stored in a stable struct
impl BoundedStorable for Saloon {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

// another trait that must be implemented for a struct that is stored in a stable struct
impl BoundedStorable for SaloonService {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    static SALOON_STORAGE: RefCell<StableBTreeMap<u64, Saloon, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));

    static SERVICE_STORAGE: RefCell<StableBTreeMap<u64, SaloonService, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct SaloonPayload {
    name: String,
    location: String,
    saloon_url: String,
}

// Struct definition for ServicesPayload
#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct ServicePayload {
    service_name: String,
    service_description: String,
}

// Error enumeration for more informative error handling
#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error {
    NotFound { msg: String },
    NotAuthorized { msg: String },
    BadRequest { msg: String },
}

// Helper function to perform insert
fn do_insert(saloon: &Saloon) {
    SALOON_STORAGE.with(|storage| storage.borrow_mut().insert(saloon.id, saloon.clone()));
}

// Helper function to get a saloon by ID
fn _get_saloon(id: &u64) -> Option<Saloon> {
    SALOON_STORAGE.with(|storage| storage.borrow().get(id))
}

// Helper function to validate saloon payload
fn validate_saloon_payload(payload: &SaloonPayload) -> Result<(), Error> {
    if payload.name.trim().is_empty() {
        return Err(Error::BadRequest { msg: "Saloon name cannot be empty".into() });
    }
    if payload.location.trim().is_empty() {
        return Err(Error::BadRequest { msg: "Saloon location cannot be empty".into() });
    }
    if payload.saloon_url.trim().is_empty() {
        return Err(Error::BadRequest { msg: "Saloon URL cannot be empty".into() });
    }
    Ok(())
}

// Helper function to validate service payload
fn validate_service_payload(payload: &ServicePayload) -> Result<(), Error> {
    if payload.service_name.trim().is_empty() {
        return Err(Error::BadRequest { msg: "Service name cannot be empty".into() });
    }
    if payload.service_description.trim().is_empty() {
        return Err(Error::BadRequest { msg: "Service description cannot be empty".into() });
    }
    Ok(())
}

// Function to log actions
fn log_action(action: &str, saloon_id: Option<u64>, details: &str) {
    ic_cdk::println!("Action: {}, Saloon ID: {:?}, Details: {}", action, saloon_id, details);
}

// Get all saloons with pagination
#[ic_cdk::query]
fn get_saloons(offset: u64, limit: u64) -> Vec<Saloon> {
    SALOON_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .skip(offset as usize)
            .take(limit as usize)
            .map(|(_, item)| item.clone())
            .collect()
    })
}

// Get a specific saloon by ID
#[ic_cdk::query]
fn get_saloon(id: u64) -> Result<Saloon, Error> {
    match _get_saloon(&id) {
        Some(saloon) => Ok(saloon),
        None => Err(Error::NotFound { msg: format!("A saloon with id={} not found", id) }),
    }
}

// Create a new saloon
#[ic_cdk::update]
fn add_saloon(payload: SaloonPayload) -> Result<Saloon, Error> {
    validate_saloon_payload(&payload)?;
    let id = ID_COUNTER.with(|counter| {
        let current_value = *counter.borrow().get();
        counter.borrow_mut().set(current_value + 1);
        current_value + 1
    });
    let saloon = Saloon {
        owner: caller().to_string(),
        id,
        name: payload.name,
        location: payload.location,
        saloon_url: payload.saloon_url,
        saloonservices: Vec::new(),
        created_at: time(),
        updated_at: None,
    };
    do_insert(&saloon);
    log_action("add_saloon", Some(id), "Saloon created");
    Ok(saloon)
}

// Add a service to a specific saloon
#[ic_cdk::update]
fn add_services_saloon(id: u64, payload: ServicePayload) -> Result<Saloon, Error> {
    validate_service_payload(&payload)?;
    let mut saloon = match _get_saloon(&id) {
        Some(saloon) => saloon,
        None => return Err(Error::NotFound { msg: format!("Couldn't find a saloon with id={}", id) }),
    };

    if saloon.owner != caller().to_string() {
        return Err(Error::NotAuthorized { msg: "You are not the owner".into() });
    }

    let service = SaloonService {
        service_name: payload.service_name,
        service_description: payload.service_description,
        created_at: time(),
        updated_at: None,
    };

    saloon.saloonservices.push(service);
    do_insert(&saloon);
    log_action("add_services_saloon", Some(id), "Service added to saloon");
    Ok(saloon)
}

// Update a specific saloon
#[ic_cdk::update]
fn update_saloon(id: u64, payload: SaloonPayload) -> Result<Saloon, Error> {
    validate_saloon_payload(&payload)?;
    let mut saloon = match _get_saloon(&id) {
        Some(saloon) => saloon,
        None => return Err(Error::NotFound { msg: format!("Couldn't find a saloon with id={}", id) }),
    };

    if saloon.owner != caller().to_string() {
        return Err(Error::NotAuthorized { msg: "You are not the owner".into() });
    }

    saloon.name = payload.name;
    saloon.location = payload.location;
    saloon.saloon_url = payload.saloon_url;
    saloon.updated_at = Some(time());
    do_insert(&saloon);
    log_action("update_saloon", Some(id), "Saloon updated");
    Ok(saloon)
}

// Delete a specific saloon
#[ic_cdk::update]
fn delete_saloon(id: u64) -> Result<Saloon, Error> {
    let saloon = match SALOON_STORAGE.with(|storage| storage.borrow_mut().remove(&id)) {
        Some(saloon) => saloon,
        None => return Err(Error::NotFound {
            msg: format!("Couldn't delete a saloon with id={}. Saloon not found.", id),
        }),
    };

    if saloon.owner != caller().to_string() {
        SALOON_STORAGE.with(|storage| storage.borrow_mut().insert(id, saloon.clone())); // Revert deletion if not authorized
        return Err(Error::NotAuthorized {
            msg: "You are not the owner".into(),
        });
    }

    log_action("delete_saloon", Some(id), "Saloon deleted");
    Ok(saloon)
}


// Delete a specific service from a saloon
#[ic_cdk::update]
fn delete_service_saloon(saloon_id: u64, service_name: String) -> Result<Saloon, Error> {
    let mut saloon = match _get_saloon(&saloon_id) {
        Some(saloon) => saloon,
        None => return Err(Error::NotFound {
            msg: format!("Couldn't find a saloon with id={}", saloon_id),
        }),
    };

    if saloon.owner != caller().to_string() {
        return Err(Error::NotAuthorized {
            msg: "You are not the owner".into(),
        });
    }

    let initial_length = saloon.saloonservices.len();
    saloon.saloonservices.retain(|s| s.service_name != service_name);

    if saloon.saloonservices.len() == initial_length {
        return Err(Error::NotFound {
            msg: format!("Service with name {} not found in saloon id={}", service_name, saloon_id),
        });
    }

    do_insert(&saloon);
    log_action("delete_service_saloon", Some(saloon_id), "Service deleted from saloon");
    Ok(saloon)
}

// Search saloon by name
#[ic_cdk::query]
fn search_by_name(name: String) -> Vec<Saloon> {
    SALOON_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .filter(|(_, item)| item.name == name)
            .map(|(_, item)| item.clone())
            .collect()
    })
}

// Search saloon by location
#[ic_cdk::query]
fn search_by_location(location: String) -> Vec<Saloon> {
    SALOON_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .filter(|(_, item)| item.location == location)
            .map(|(_, item)| item.clone())
            .collect()
    })
}

// need this to generate candid
ic_cdk::export_candid!();
