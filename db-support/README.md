# Simple database backend setup using Docker, Clickhouse and Grafana.

## Usage
- Ensure docker (docker compose) is available and installed on the system you would like to backup/send all experimental data to.
- Download the `docker-compose` and `init.sql` files into an appropriate folder/location
- In that location, run `docker compose up -d`
- In a web browser, you should be to access Grafana at either `0.0.0.0:9000` if you are running this locally, or at the IP address of the server you are running it on.
- To get access to your stored data, within the Grafana menu > connections > add new data souce > clickhouse > enter your port & credentials of the clickhouse database. **Note** use `clickhouse` as the server/address. 
- Once the connection is made, you can explore the data in the explore tab. 