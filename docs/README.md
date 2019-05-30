# REview web API

The REview web API is an HTTP API to interact with the central repo database. The API is defined by the [Swagger](http://swagger.io/specification/) definition in `swagger.yaml`.

It consists of two files in this repository:

- `swagger.yaml` A Swagger definition of the API.
- `swagger.html` An HTML document automatically generated from `swagger.yaml`.

## Viewing the API documentation
You can view `swagger.html` with your browser. If you want to view `swagger.yaml` with [Swagger Editor](https://github.com/swagger-api/swagger-editor), follow the steps below:

1. Pull [Docker Image](https://hub.docker.com/r/swaggerapi/swagger-editor/) from Docker Hub
```
    docker pull swaggerapi/swagger-editor
```
2. Run a container of swagger editor
```
    docker run -d -p <port number>:8080 swaggerapi/swagger-editor
```

3. Access `localhost:<port number you specified in step 2.>` from a browser
4. Click `[File]-[Import file]` and then choose `Swagger.yaml` file
