const { Client } = require('pg');

class SerinClient {
  constructor(config) {
    this.client = new Client(config);
  }

  async connect() {
    await this.client.connect();
  }

  async query(sql, params = []) {
    return this.client.query(sql, params);
  }

  async end() {
    await this.client.end();
  }
}

module.exports = { SerinClient }; 