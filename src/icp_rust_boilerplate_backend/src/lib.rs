#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Saloon {
    id: u64,
    name: String,
    location: String,
    saloon_url: String,
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

// another trait that must be implemented for a struct that is stored in a stable struct
impl BoundedStorable for Saloon {
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
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct SaloonPayload {
    name: String,
    location: String,
    saloon_url: String,
}

#[ic_cdk::query]
fn get_saloon(id: u64) -> Result<Saloon, Error> {
    match _get_saloon(&id) {
        Some(shoe) => Ok(shoe),
        None => Err(Error::NotFound {
            msg: format!("a saloon with id={} not found", id),
        }),
    }
}

#[ic_cdk::update]
fn add_saloon(payload: SaloonPayload) -> Option<Saloon> {
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("cannot increment id counter");
    let saloon = Saloon {
        id,
        name: payload.name,
        location: payload.location,
        saloon_url: payload.saloon_url,
        created_at: time(),
        updated_at: None,
    };
    do_insert(&saloon);
    Some(saloon)
}

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

// helper method to perform insert.
fn do_insert(saloon: &Saloon) {
    SALOON_STORAGE.with(|service| service.borrow_mut().insert(saloon.id, saloon.clone()));
}

#[ic_cdk::update]
fn delete_saloon(id: u64) -> Result<Saloon, Error> {
    match SALOON_STORAGE.with(|service| service.borrow_mut().remove(&id)) {
        Some(saloon) => Ok(saloon),
        None => Err(Error::NotFound {
            msg: format!(
                "couldn't delete a message with id={}. message not found.",
                id
            ),
        }),
    }
}

#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error {
    NotFound { msg: String },
}

// a helper method to get a message by id. used in get_message/update_message
fn _get_saloon(id: &u64) -> Option<Saloon> {
    SALOON_STORAGE.with(|service| service.borrow().get(id))
}

// need this to generate candid
ic_cdk::export_candid!();