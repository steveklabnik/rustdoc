import Ember from 'ember';

export default Ember.Route.extend({
    model(params) {
        return Ember.$.getJSON('/data.json').then(data => {
            this.get('store').push(data);
            return this.get('store').peekRecord('crate', params.crate_name);
        });
    }
});
