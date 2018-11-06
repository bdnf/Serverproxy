extern crate hyper;
extern crate pretty_env_logger;

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

use hyper::{Body, Chunk, Client, Method, Request, Response, Server, StatusCode, header};
use hyper::client::HttpConnector;

use hyper::client::ResponseFuture;
use hyper::service::service_fn;
//use hyper::rt::{self, Future};
use std::net::SocketAddr;

extern crate futures;
use futures::{future, Future, Stream};
//use futures::future;


use std::collections::HashMap;

extern crate url;
use url::form_urlencoded;

extern crate jsonwebtoken as jwt;
use jsonwebtoken::{encode, decode,decode_header, Header, Algorithm, Validation};
use jsonwebtoken::errors::{ErrorKind};



#[derive(Serialize, Deserialize, Debug)]
struct User {
    id: String,
    username: String,
    profileAccessLevel: String
}
#[derive(Serialize, Deserialize, Debug)]
struct User2 {
    username: String
}


mod models;
mod parser;
//use crate::parser::resources::INDEX;
use crate::parser::resources:: *;
//mod resources;



fn response_examples(mut req: Request<Body>, client: &Client<HttpConnector>, server_addr: SocketAddr)
    -> Box<Future<Item=Response<Body>, Error=hyper::Error> + Send> {

    let key: String = "jhfdlksjlkfjlksdjfljsdlj".to_owned(); //jwt token secret key

    let server_addr_clone = server_addr.clone();
    let uri_string = format!("http://{}{}",
        server_addr_clone,
        req.uri().path_and_query().map(|x| x.as_str()).unwrap_or(""));
    let uri = uri_string.parse().unwrap();
    *req.uri_mut() = uri;

    let new_res = parser::match_request(&req);
    //new_res


    //matching against specific routes
    match (req.method(), req.uri().path()) {

           (&Method::GET, HOMEPAGE) | (&Method::GET, "/index.html") => {
               let body = Body::from(INDEX);
               Box::new(future::ok(Response::new(body)))
           },
           (&Method::GET, "/test") => {
               // build the request and change the path
               let request_uri = req.uri();
               let upstream_uri = format!("http://{}:{}{}",
                   req.uri().host().unwrap(),
                   req.uri().port().unwrap(),
                   "/"
                   ).parse().unwrap();

                println!("{:?}", upstream_uri);
                *req.uri_mut() = upstream_uri;


                println!("{:?}", req);
                let web_res_future = client.request(req);


                Box::new(web_res_future)

           },
           (&Method::GET, "/test.html") => {
               // Run a web query against the web api below

               // build the request
               let req = Request::builder()
                   .method(Method::POST)
                   .uri(uri_string)
                   .body(LOWERCASE.into())
                   .unwrap();
               // use the request with client
               let web_res_future = client.request(req);

               Box::new(web_res_future.map(|web_res| {
                   // return the response that came from the web api and the original text together
                   // to show the difference
                   let body = Body::wrap_stream(web_res.into_body().map(|b| {
                       Chunk::from(format!("<b>before</b>: {}<br><b>after</b>: {}",
                                           std::str::from_utf8(LOWERCASE).unwrap(),
                                           std::str::from_utf8(&b).unwrap()))
                   }));

                   Response::new(body)
               }))
           },
           (&Method::POST, "/post") => {
               Box::new(req.into_body().concat2().map(|b| {
                 let params = form_urlencoded::parse(b.as_ref()).into_owned().collect::<HashMap<String, String>>();
                 println!("{:?}",params );
                 // Validate the request parameters, returning
                 // early if an invalid input is detected.
                 let name = if let Some(n) = params.get("name") {
                     n
                 } else {
                     println!("{:?}", params.get("name"));
                     return Response::builder()
                         .status(StatusCode::UNPROCESSABLE_ENTITY)
                         .body(MISSING.into())
                         .unwrap();
                 };
                 let number = if let Some(n) = params.get("number") {
                     if let Ok(v) = n.parse::<f64>() {
                         v
                     } else {
                         return Response::builder()
                             .status(StatusCode::UNPROCESSABLE_ENTITY)
                             .body(NOTNUMERIC.into())
                             .unwrap();
                     }
                 } else {
                     return Response::builder()
                         .status(StatusCode::UNPROCESSABLE_ENTITY)
                         .body(MISSING.into())
                         .unwrap();
                 };

                 // Render the response. This will often involve
                 // calls to a database or web service, which will
                 // require creating a new stream for the response
                 // body. Since those may fail, other error
                 // responses such as InternalServiceError may be
                 // needed here, too.
                 let body = format!("Hello {}, your number is {}", name, number);
                 Response::new(body.into())
             }))
           },
           (&Method::POST, "/web_api") => {
               // A web api to run against. Uppercases the body and returns it back.
               let body = Body::wrap_stream(req.into_body().map(|chunk| {
                   // uppercase the letters
                   let upper = chunk.iter().map(|byte| byte.to_ascii_uppercase())
                       .collect::<Vec<u8>>();
                   Chunk::from(upper)
               }));
               Box::new(future::ok(Response::new(body)))
           },
           (&Method::GET, "/json") => {
               let data = vec!["foo", "bar"];
               let res = match serde_json::to_string(&data) {
                   Ok(json) => {
                       let users:User = serde_json::from_str(&json).unwrap();
                       println!("{:?}", users );
                       // return a json response
                       Response::builder()
                           .header(header::CONTENT_TYPE, "application/json")
                           .body(Body::from(json))
                           .unwrap()
                   }
                   // This is unnecessary here because we know
                   // this can't fail. But if we were serializing json that came from another
                   // source we could handle an error like this.
                   Err(e) => {
                       eprintln!("serializing json: {}", e);

                       Response::builder()
                           .status(StatusCode::INTERNAL_SERVER_ERROR)
                           .body(Body::from("Internal Server Error"))
                           .unwrap()
                   }
               };

               Box::new(future::ok(res))
           },
           (&Method::POST, "/json") => {
               let chunks = vec![
                "hello",
                " ",
                "world",
            ];

            let stream = futures::stream::iter_ok::<_, ::std::io::Error>(chunks);

            let body = Body::wrap_stream(stream);


             Box::new(future::ok(Response::builder()
                 .header(header::CONTENT_TYPE, "application/json")
                 .body(body)
                 .unwrap()
             ))
           },
           (&Method::GET, "/api/auth/signin") => {
                //let data: Vec<&str> = value.split("/").skip(1).collect();

               Box::new(client.request(req))

           },
           (&Method::POST, "/api/auth/signin") => {

               println!("{:?}", req.headers());

           /*
               let newBody = req.body_mut().map_err(|_| ()).fold(vec![], |mut acc, chunk| {
                 acc.extend_from_slice(&chunk);
                 Ok(acc)
               }).and_then(|v| String::from_utf8(v).map_err(|_| ()).wait().unwrap());

               Box::new(future::ok(Response::builder()
                   .header(header::CONTENT_TYPE, "application/json")
                   .body(Body::from(newBody))
                   .unwrap()
               ))
           */


               Box::new(client.request(req))
           },
           //matching complex routes
           (&Method::GET, value) => {
               println!("{:?}", req.headers().get("authorization"));
               println!("{:?}", req.body());
               let data: Vec<&str> = value.split("/").skip(1).collect();
               //let data = vec!["user", "pass"];
               //let index: Option<usize> = Some(data.iter().position(|&x| x == "user").unwrap());

               if data.len() >=2 {
               match (data[0], data[1])  {
                    ("api","users",) => {
                        println!("{:?}", req);
                        Box::new(client.request(req))},
                    ("api","messages",) => {
                        println!("{:?}", req.headers().get("authorization"));
                        let mut token = req.headers().get("authorization").unwrap().to_str();

                        match token {
                            Ok(x) => {
                                // HMAC using SHA-256
                                let auth_header: Vec<&str> = x.split_whitespace().collect();
                                println!("token for decoding is{}", auth_header[1]);
                                let token_data = match decode::<User>(auth_header[1], key.as_ref(), &Validation::new(Algorithm::HS256)) {
                                    Ok(c) => c,
                                    Err(err) => match *err.kind() {
                                        ErrorKind::InvalidToken => panic!(), // Example on how to handle a specific error
                                        _ => panic!()
                                    }
                                };

                                println!("Token data is{:?}", token_data);
                            }
                            _ => ()
                        };
Box::new(client.request(req))
                    },

                    (_,_) => {
                        let idx = match data.iter().position(|&x| x == "user" || x =="users") {
                            Some(index) => {

                                println!("You probably a {:?} with id: {:?}", data[index], data[index+1] );
                                let s = format!("You probably trying {:?} with id: {:?}" , data[index].to_string(), data[index+1].to_string());
                                &data[index..=index+1]
                            },
                            None => &data,
                        };

                        let res = match serde_json::to_string(idx) {

                            Ok(ref js) if idx.len() == 2 =>
                             Response::builder()
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Body::from(format!("You probably a {:?} with id: {:?}" , idx[0].to_string(), idx[1].to_string())))
                                .unwrap()
                                ,
                                Ok(ref json) if idx.len() > 2 =>
                                    // return a json response
                                    Response::builder()
                                        .header(header::CONTENT_TYPE, "application/json")
                                        .body(Body::from(json.clone()))
                                        .unwrap()
                                        ,


                             Ok(_) =>
                                 Response::builder()
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(Body::from(format!("You probably a requested a /")))
                                    .unwrap()
                                    ,
                            // This is unnecessary here because we know
                            // this can't fail. But if we were serializing json that came from another
                            // source we could handle an error like this.
                            Err(e) => {
                                eprintln!("serializing json: {}", e);

                                Response::builder()
                                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                                    .body(Body::from("Internal Server Error"))
                                    .unwrap()
                            }
                        };

                        Box::new(future::ok(res))
                    }
               }

           }  else  {
                Box::new(future::ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Error reaching un-existing route"))
                .unwrap()))
            }
       }
           _ => {
               // Return 404 not found response.
               let body = Body::from(NOTFOUND);
               Box::new(future::ok(Response::builder()
                                            .status(StatusCode::NOT_FOUND)
                                            .body(body)
                                            .unwrap()))
           }
       }
   }



fn main() {
    // let user = models::User {
    //     name: "Andre".to_string(),
    //     group: Some(models::Group {
    //         group_name: "Admin".to_string(),
    //         allowed_verbs: vec!["GET".to_string(), "POST".to_string(), "DELETE".to_string()],
    //     }),
    //     id: 10001,
    // };
    // let jjson = serde_json::to_string(&user).expect("Couldn't serialize config");
    // println!("{}", jjson);

    //ROUTES.iter().map(|&x| { print!("{:?}", x );});
    for (i,r) in ROUTES.iter().enumerate() {
        print!("{:?} ", r );

    }

    //println!("{}", ROUTES.iter().fold(String::new(), |acc, &arg| acc + arg));



    pretty_env_logger::init();

    let in_addr = ([127, 0, 0, 1], 30001).into();
    let server_addr: SocketAddr = ([127, 0, 0, 1], 1331).into();
    //let backup_addr: SocketAddr = ([127, 0, 0, 1], 1331).into();


    hyper::rt::run(future::lazy(move || {
    let client_main = Client::new();

    // new_service is run for each connection, creating a 'service'
    // to handle requests for that specific connection.

    let new_service = move || {
            // Move a clone of `client` into the `service_fn`.
            let client = client_main.clone();
            service_fn(move |req| {
                response_examples(req, &client, server_addr)
            })
        };

/*
    let new_service = move || {
        let client = client_main.clone();
        // This is the `Service` that will handle the connection.
        // `service_fn_ok` is a helper to convert a function that
        // returns a Response into a `Service`.
        service_fn(move |mut req| {
            let uri_string = format!("http://{}{}",
                server_addr_clone,
                req.uri().path_and_query().map(|x| x.as_str()).unwrap_or(""));
            let uri = uri_string.parse().unwrap();
            *req.uri_mut() = uri;
            //println!("{:?}", req);
            //println!("{:?}", req.method());
            match req.method() {
                &Method::POST => {
                    //println!("{:?}", req.method());
                    //client.request(req);
                    let uri_string = format!("http://{}{}",
                        backup_addr_clone,
                        req.uri().path_and_query().map(|x| x.as_str()).unwrap_or(""));
                    let uri = uri_string.parse().unwrap();


                    let (mut parts, body) = req.into_parts();
                    parts.method = Method::GET;
                    parts.uri = uri;
                    let request = Request::from_parts(parts, body);


                    println!("{:?}", request);
                    client.request(request)
                },
                &Method::GET => {
                    println!("{:?}", req);
                    client.request(req)
                    },
                _ => {
                    // Return 404 not found response.
                    let body = Body::from(NOTFOUND);
                    let resp404 = Request::builder()
                                     //.status(StatusCode::NOT_FOUND)
                                     .body(body)
                                     .unwrap();
                    println!("{:?}", req.method());
                    client.request(resp404)
                    }


            }
            //print!("{:?}", req.type);
            //client.request(req)
        })
    };
*/
    let server = Server::bind(&in_addr)
        .serve(new_service)
        .map_err(|e| eprintln!("server error: {}", e));


        let my_claims = User2 {

              username: "testname8".to_owned(),

          };
          /*
         ISSUE: panicked at 'called `Result::unwrap()` on an `Err` value: Error(ExpiredSignature)'
        let key = "secret".to_owned();

        let token = encode(&Header::default(), &my_claims, key.as_ref()).unwrap();
        println!("{:?}", token);

        let token_data = decode::<User2>(&token, key.as_ref(), &Validation::default()).unwrap();
        //let token_data = decode_header(&token);
        println!("{:?}", token_data);
        //println!("{:?}", token_data.header);
        */

    println!("Listening on http://{}", in_addr);
    println!("Proxying on http://{}", server_addr);

    server
    }));
}
