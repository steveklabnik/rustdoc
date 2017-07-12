import fetch from 'fetch';
import Ember from 'ember';

export default Ember.Route.extend({
  beforeModel() {
    return fetch('data.json')
      .then(response => response.json())
      .then(data => this.store.push(data));
    },
});
