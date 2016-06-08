#[macro_use] 
extern crate nickel;
extern crate rustc_serialize;
extern crate rusqlite;

use nickel::{Nickel, JsonBody, HttpRouter, Request, Response, MiddlewareResult, MediaType,  StaticFilesHandler};
use rusqlite::Connection;

use rustc_serialize::json::{Json, ToJson};
use std::prelude::*;


// Use postgres for database

#[derive(RustcDecodable, RustcEncodable, Debug)]
struct TodoItem {
    id: Option<usize>,
    text: Option<String>,
    completed: Option<bool>
}

#[derive(RustcDecodable, RustcEncodable, Debug)]
struct JsonRequest {
    todo: TodoItem
}

fn main() {


    let dbconn = Connection::open("app.db").unwrap();

    let mut server = Nickel::new();
    let mut router = Nickel::router();

    router.get("/todo/", middleware! { |request, response|
        // List all items

        format!("Hello from GET /users")

    });
    
    router.get("/users/:id", middleware! { |request, response|

        let id = request.param("id").unwrap();

        format!("Trying to retreive TODO number {}", id)

    });
    
    router.post("/todo/", middleware! { |request, response|
        // Insert stuff and return what we inserted

        let jsonreq = request.json_as::<JsonRequest>().unwrap();

        let todoitem = jsonreq.todo;

        let text = todoitem.text.unwrap().to_string();

        //let newtodo = request.json_as::<TodoItem>().unwrap();

        //let text = newtodo.text.unwrap().to_string();

        format!("Trying to create a new TODO list with text {:?}", text)

    });

    
    router.put("/users/:id", middleware! { |request, response|
        // Update the item

        let id = request.param("id").unwrap();

        format!("Trying to update TODO number {}", id)

    });
    
    router.delete("/users/:id", middleware! { |request, response|
        // Delete the item

        let id = request.param("id").unwrap();

        format!("Trying to delete TODO number {}", id)

    });




    server.utilize(router);
    server.utilize(StaticFilesHandler::new("assets/"));
    server.listen("127.0.0.1:6767");
}