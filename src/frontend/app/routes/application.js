import Ember from 'ember';

export default Ember.Route.extend({
    beforeModel() {
        return this._populateStore();
    },

    _populateStore() {
        return Ember.$.getJSON('/data.json').then(data => {
            this.store.push(data);
        });
    }
});