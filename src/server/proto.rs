

    // 1:   get
    // 2:   give (by default only give the first 1k bytes, and the first 100 fields)
    // 3:   get_val (to get more than 1k bytes)
    // 4:   give_val
    // 5:   unauthorized
    // 6:   sub
    // 7:   unsub
    // 8:   update_request
    // 9:   update_deny
    // 10:  update
    // 11:  delete
    // 12:  create
    // 13:  created_id
    // 14:  unsupported_version
    // 15:  get_and_sub
    // 16:  get_fields
    // 17:  error

pub static MSG_GET: u8 = 1;
pub static MSG_GIVE: u8 = 2;
pub static MSG_GET_VAL: u8 = 3;
pub static MSG_GIVE_VAL: u8 = 4;
pub static MSG_UNAUTHORIZED: u8 = 5;
pub static MSG_SUB: u8 = 6;
pub static MSG_UNSUB: u8 = 7;
pub static MSG_UPDATE_REQUEST: u8 = 8;
pub static MSG_UPDATE_DENY: u8 = 9;
pub static MSG_UPDATE: u8 = 10;
pub static MSG_DELETE: u8 = 11;
pub static MSG_CREATE: u8 = 12;
pub static MSG_CREATED_ID: u8 = 13;
pub static MSG_UNSUPPORTED_VERSION: u8 = 14;
pub static MSG_GET_AND_SUB: u8 = 15;
pub static MSG_GET_FIELDS: u8 = 16;
pub static MSG_ERROR: u8 = 17;



//use std::panic::update_hook;
use std::vec;

use crate::server::itemstore;
use crate::server::itemstore::encode;
use crate::error;

pub enum Response {
    One(Vec<u8>),
    All(Vec<u8>),
    AllSubbed(u64, Vec<u8>),
    None,
}

pub struct Message {
    msg: Vec<u8>,
    version: u8,
    cmd: u8,
    index: usize,
}

impl Message {
    pub fn new(vec: Vec<u8>) -> Message {

        // this weired version does not panic, while others do for some unknown reason  
        let mut count = 0;

        let mut cmd: u8 = 0;
        let mut version: u8 = 0;

        for i in vec.clone() {
            if count == 0 {
                version = i;
            }
            if count == 1 {
                cmd = i
            }

            count += 1;
        };

        Message { msg: vec, version: version, cmd: cmd, index: 2 }
    }

    pub fn get_id(&mut self) -> u64 {
       self.get_u64()
    }

    pub fn get_u32(&mut self) -> u32 {
            let tmp = &self.msg[self.index..self.index + 4];
            let num: u32 = u32::from_be_bytes([tmp[0], tmp[1], tmp[2], tmp[3]]);
            self.index += 4;

            return num;
    }

    pub fn get_u64(&mut self) -> u64 {
            let tmp = &self.msg[self.index..self.index + 8];
            let num: u64 = u64::from_be_bytes([tmp[0], tmp[1], tmp[2], tmp[3], tmp[4], tmp[5], tmp[6], tmp[7]]);
            self.index += 8;

            return num;
    }
    
    pub fn get_bytes(&mut self, n: usize) -> Vec<u8>{
        let tmp = &self.msg[self.index..self.index + n];
        self.index += n;

        return tmp.to_vec();
    }

}

const VERSION: u8 = 1;

pub async fn handle_mize_message(
        mut message: Message,
        itemstore: &itemstore::Itemstore,
    ) -> Vec<Response> {

    let old_message = message.msg.clone();


    //println!("message: {:?}", message);
    //let version = message.clone().into_iter().nth(0).expect("message has no 0th byte");
    //let cmd = message.clone().into_iter().nth(1).expect("message has no 1th byte");
    //println!("after chaos");

    // for some weired reason this stupid solution does not panic, while the obove one and vec[0]
    // do
    let mut count = 0;

    let mut cmd: u8 = 0;
    let mut version: u8 = 0;

    for i in old_message.clone() {
        if count == 0 {
            version = i;
        }
        if count == 1 {
            cmd = i
        }

        count += 1;
    };

    //println!("VERSION: {}", version);
    //println!("CMD: {}", cmd);

    match cmd {
        //get
        1 => {
            //let id_bytes = *&message[2..9].to_owned().clone();
            let tmp = &old_message[2..10];
            let id: u64 = u64::from_be_bytes([tmp[0], tmp[1], tmp[2], tmp[3], tmp[4], tmp[5], tmp[6], tmp[7]]);

            // answer:
            // u8: version
            // u8: cmd (2 for give)
            // u64: id
            // u32: num_of_fields
            // as often as num_of_fields:
                // u64: key_len
                // key_len: key
                // u64: val_len
                // val_len: val
            let mut answer: Vec<u8> = vec![VERSION, 2];
            answer.extend(id.to_be_bytes());

            let mut item = match itemstore.get(id).await {
                Ok(item) => item,
                Err(err) => {return vec![err.to_message()]},
            };

            let num_of_fields = item.len() as u32;
            answer.extend(num_of_fields.to_be_bytes());

            for field in item {
                let key_len = field[0].len() as u32;
                answer.extend(key_len.to_be_bytes());
                answer.extend(field[0].clone());

                let val_len = field[1].len() as u32;
                answer.extend(val_len.to_be_bytes());
                answer.extend(field[1].clone());
            }
            return vec![Response::One(answer)];
        },
        2 => {return Vec::new()},
        3 => {return Vec::new()},
        4 => {return Vec::new()},
        5 => {return Vec::new()},
        6 => {return Vec::new()},
        7 => {return Vec::new()},

        //update_request
        8 => {
            let mut answer: Vec<u8> = message.msg.clone();
            answer[1] = 10;

            let id = message.get_id();

            let mut item = match itemstore.get(id).await {
                Ok(item) => item,
                Err(err) => {return vec![err.to_message()]},
            };

            let num_of_updates = message.get_u32();

            for i in 0..num_of_updates as usize {
                let key_len = message.get_u32();
                let key = message.get_bytes(key_len as usize);
                //println!("KEY: {}", String::from_utf8(key.clone()).expect("here utf-8"));

                let update_len = message.get_u32();
                let update = message.get_bytes(update_len as usize);

                let mut found = false;
                for a in 0..item.len() as usize {
                    if item[a][0] == key {
                        item[a][1] = apply_update(&item[a][1], &update);
                        found = true;
                        break;
                    }
                }
                if found == false {
                    let index = item.len() +1;
                    //item[index][1] = apply_update(&item[index][1], &update)
                    item.push([key.clone(), apply_update(&item[index][1], &update)])
                }
            }

            if let Err(err) = itemstore.update(id, item).await {
                return vec![err.to_message()];
            };

            return vec![Response::All(answer)];
        },
        9 => {return Vec::new()},
        10 => {return Vec::new()},

        //delete
        11 => {
            let response: Vec<Response> = Vec::new();
            let tmp = &old_message[2..10];
            let id: u64 = u64::from_be_bytes([tmp[0], tmp[1], tmp[2], tmp[3], tmp[4], tmp[5], tmp[6], tmp[7]]);
            if id == 0 {
                let err = error::MizeError{
                    kind: "don't know yet".to_string(),
                    code: 100,
                    message: "You can't delete item number 0. This item contains mandatory Config for the Server.".to_string(),
                };
                return vec![err.to_message()];
            }

            if let Err(err) = itemstore.delete(id).await {
                return vec![err.to_message()];
            }
            return response;
        },

        //create
        12 => {
            let num_of_fields = message.get_u32();
            let mut item: Vec<[Vec<u8>; 2]> = Vec::new();

            let mut index = 6;

            for i in 0..num_of_fields {
                let key_len = message.get_u32();
                let key = message.get_bytes(key_len as usize);

                let val_len = message.get_u32();
                let val = message.get_bytes(val_len as usize);
                
                item.push([key, val]);
            }

            if let Err(err) = itemstore.create(item).await {
                return vec![err.to_message()];
            };

            let anser: Vec<u8> = vec![1,1];
            return vec![Response::None];
        },
        13 => {return Vec::new()},
        14 => {return Vec::new()},
        _ => {return Vec::new()},
    }


}

// just like all of this crate, definetly could be done with less clones().... and better error
// handling
pub fn apply_update(val: &Vec<u8>, updates: &Vec<u8>) -> Vec<u8>{
    println!("VAL: {:?}", val.clone());
    println!("Update: {:?}", updates.clone());
    let not_enough_bytes = "not enough bytes in update";
    let mut val = val.clone();
    let mut update_iter = updates.clone().into_iter();
    while true {
        if let Some(operation) = update_iter.next(){
            let mut new_val: Vec<u8> = Vec::new();
            let mut val_iter = val.clone().into_iter();
            match operation {
                //r,start:u32,stop:u32,bytes start..stop
                0 => {
                    //get start and stop
                    let mut start_bytes: [u8; 4] = [0,0,0,0];
                    for i in 0..4 {start_bytes[i] = update_iter.next().expect(not_enough_bytes);};
                    let start = u32::from_be_bytes(start_bytes);

                    let mut stop_bytes: [u8; 4] = [0,0,0,0];
                    for i in 0..4 {stop_bytes[i] = update_iter.next().expect(not_enough_bytes);};
                    let stop = u32::from_be_bytes(stop_bytes);

                    //add the stuff before
                    for i in 0..start {new_val.push(val_iter.next().expect(not_enough_bytes));};

                    //add the new stuff
                    for i in start..stop {new_val.push(update_iter.next().expect(not_enough_bytes));};

                    //skip all the bytes that should be replaced 
                    for i in 0..stop-start {val_iter.next().expect(not_enough_bytes);}

                    //add stuff after
                    for byte in val_iter {
                        new_val.push(byte);
                    }
                },
                //i,start:u32, stop:u32, bytes stop-start
                1 => {
                    //get start and stop
                    let mut start_bytes: [u8; 4] = [0,0,0,0];
                    for i in 0..4 {start_bytes[i] = update_iter.next().expect(not_enough_bytes);};
                    let start = u32::from_be_bytes(start_bytes);

                    let mut stop_bytes: [u8; 4] = [0,0,0,0];
                    for i in 0..4 {stop_bytes[i] = update_iter.next().expect(not_enough_bytes);};
                    let stop = u32::from_be_bytes(stop_bytes);

                    //add the stuff before
                    for i in 0..start {new_val.push(val_iter.next().expect(not_enough_bytes));};

                    //add the new stuff
                    for i in start..stop {new_val.push(update_iter.next().expect(not_enough_bytes));};

                    //add stuff after
                    for byte in val_iter {
                        new_val.push(byte);
                    }

                    //while true {
                        //if let Some(byte) = update_iter.next() {
                            //new_val.push(byte);
                        //} else {break;}
                    //}
                },
                //d,start:u32,stop:u32
                2 => {
                    //get start and stop
                    let mut start_bytes: [u8; 4] = [0,0,0,0];
                    for i in 0..4 {start_bytes[i] = update_iter.next().expect(not_enough_bytes);};
                    let start = u32::from_be_bytes(start_bytes);

                    let mut stop_bytes: [u8; 4] = [0,0,0,0];
                    for i in 0..4 {stop_bytes[i] = update_iter.next().expect(not_enough_bytes);};
                    let stop = u32::from_be_bytes(stop_bytes);

                    //add the stuff before
                    for i in 0..start {new_val.push(val_iter.next().expect("not_enough_bytes in val_iter"));};

                    //skip all the bytes that should be replaced 
                    for i in 0..stop-start {let h = val_iter.next().expect("not_enough_bytes in val_iter"); println!("VAL ITER: {}", h);}

                    //add stuff after
                    for byte in val_iter {
                        new_val.push(byte);
                    }
                },
                _ => {panic!("unknown update command")}
            }
            val = new_val;
        } else {break;}
    };
    return val;
}





