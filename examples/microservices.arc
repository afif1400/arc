@direction right
@theme light

# Clients
user Web "Web App"
user Mobile "Mobile App"
external Partner "Partner API"

# Edge
gateway LB "Load Balancer" [nginx]
service Auth "Auth Service" [Go, JWT]
service API "Core API" [Node.js]
service Search "Search Service" [Python]
service Notify "Notification Service" [Go]

# Data
db Main "PostgreSQL" [v16, primary]
db Analytics "ClickHouse" [analytics]
cache Session "Redis" [cluster]
queue Events "Kafka" [3 brokers]
store Files "S3 Bucket"

# Edge connections
Web -> LB: "HTTPS"
Mobile -> LB: "HTTPS"
Partner -> Auth: "OAuth2"

# Internal routing
LB -> Auth: "authenticate"
LB -> API: "route request"

# Service connections
Auth -> Session: "session lookup"
API -> Main: "CRUD"
API -> Session: "cache reads"
API -> Events: "domain events" [async]
API -> Files: "file upload"

# Async consumers
Events -> Search: "index updates" [async]
Events -> Notify: "send alerts" [async]
Events -> Analytics: "event stream" [async]

# Groups
group "AWS us-east-1" {
  LB
  group "VPC" {
    group "Public Subnet" {
      LB
    }
    group "Private Subnet" {
      Auth, API, Search, Notify
      Main, Session, Events, Files
    }
  }
  Analytics
}
