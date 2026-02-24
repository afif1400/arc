@direction right
@theme light

user Client "Web Browser"
gateway Nginx "Load Balancer" [2 instances]
service API "REST API" [Go, v2.1]
db Postgres "Main Database" [v16]
cache Redis "Session Cache" [cluster]

Client -> Nginx: "HTTPS"
Nginx -> API: "proxy"
API -> Postgres: "read/write"
API -> Redis: "session lookup"
