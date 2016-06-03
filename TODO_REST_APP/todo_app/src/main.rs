#[macro_use] 
extern crate nickel;

use nickel::{Nickel, JsonBody, Mountable, HttpRouter, Request, Response, MiddlewareResult, MediaType,  StaticFilesHandler};

fn main() {

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

        format!("Trying to create a new TODO list")

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
    server.utilize(StaticFilesHandler::new("assets"));

    server.listen("127.0.0.1:9000");
}