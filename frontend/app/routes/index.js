import Ember from 'ember';

export default Ember.Route.extend({
  afterModel() {
    this.replaceWith('/' + this.get('store').peekAll('crate').objectAt(0).id);
  },
});
