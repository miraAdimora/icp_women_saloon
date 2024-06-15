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
    saloonservices : Vec<SaloonService>,
    created_at: u64,
    updated_at: Option<u64>,
}

// Struct definition for Services
#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct SaloonService {
    service_name : String,
    service_description : String,
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
    service_name : String,
    service_description : String,
}


// get all the available saloons
#[ic_cdk::query]
fn get_saloons() -> Vec<Saloon> {
    SALOON_STORAGE.with(|service| {
        let storage = service.borrow_mut();
        storage.iter().map(|(_, item)| item.clone()).collect()
    })
}

//get a particular saloon by its id
#[ic_cdk::query]
fn get_saloon(id: u64) -> Result<Saloon, Error> {
    match _get_saloon(&id) {
        Some(saloon) => Ok(saloon),
        None => Err(Error::NotFound {
            msg: format!("a saloon with id={} not found", id),
        }),
    }
}

// Function for creating a saloon
#[ic_cdk::update]
fn add_saloon(payload: SaloonPayload) -> Option<Saloon> {

    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("cannot increment id counter");
    let saloon = Saloon {
        owner: caller().to_string(),
        id,
        name: payload.name,
        location: payload.location,
        saloon_url: payload.saloon_url,
        saloonservices : Vec::new(),
        created_at: time(),
        updated_at: None,
    };
    do_insert(&saloon);
    Some(saloon)
}


// Function for adding services to saloon
#[ic_cdk::update]
fn add_services_saloon(id: u64, payload : ServicePayload) -> Result<Saloon, Error> {
    match SALOON_STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut saloon) => {
        // Checks if the caller is the owner of the saloon
        if saloon.owner != caller().to_string(){
            return Err(Error::NotAuthorized {
                msg: format!("You are not the owner"),
              });
            }
    else {
    let services = SaloonService {
        service_name : payload.service_name,
        service_description : payload.service_description,
        created_at: time(),
        updated_at: None,
    };

    saloon.saloonservices.push(services);
        do_insert(&saloon);
        Ok(saloon.clone())
    }}

    // If the saloon is not found, return a NotFound error
    None => Err(Error::NotFound {
        msg: format!("Couldn't update an Saloon with id={}. Saloon not found", id),
    }),
}
}

// Search saloon by Name
#[ic_cdk::query]
fn search_by_name(name: String) -> Vec<Saloon> {
    SALOON_STORAGE.with(|service| {
        let storage = service.borrow_mut();
        storage
            .iter()
            .filter(|(_, item)| item.name == name)
            .map(|(_, item)| item.clone())
            .collect()
    })
}

// Search saloon by Name
#[ic_cdk::query]
fn search_by_location(location: String) -> Vec<Saloon> {
    SALOON_STORAGE.with(|service| {
        let storage = service.borrow_mut();
        storage
            .iter()
            .filter(|(_, item)| item.location == location)
            .map(|(_, item)| item.clone())
            .collect()
    })
}

// Function that update the details of a saloon
#[ic_cdk::update]
fn update_saloon(id: u64, payload: SaloonPayload) -> Result<Saloon, Error> {
    match SALOON_STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut saloon) => {
            saloon.saloon_url = payload.saloon_url;
            saloon.location = payload.location;
            saloon.name = payload.name;
            saloon.updated_at = Some(time());
            do_insert(&saloon);
            Ok(saloon)
        }
        None => Err(Error::NotFound {
            msg: format!(
                "couldn't update a message with id={}. message not found",
                id
            ),
        }),
    }
}

// Function to delete a saloon
#[ic_cdk::update]
fn delete_saloon(id: u64) -> Result<Saloon, Error> {
    match SALOON_STORAGE.with(|service| service.borrow_mut().remove(&id)) {
        Some(saloon) => Ok(saloon),
        None => Err(Error::NotFound {
            msg: format!(
                "couldn't delete a saloon with id={}. saloon not found.",
                id
            ),
        }),
    }
}


#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error {
    NotFound { msg: String },
    NotAuthorized { msg: String },
}

// helper method to perform insert.
fn do_insert(saloon: &Saloon) {
    SALOON_STORAGE.with(|service| service.borrow_mut().insert(saloon.id, saloon.clone()));
}

// a helper method to get a message by id. used in get_message/update_message
fn _get_saloon(id: &u64) -> Option<Saloon> {
    SALOON_STORAGE.with(|service| service.borrow().get(id))
}

// need this to generate candid
ic_cdk::export_candid!();