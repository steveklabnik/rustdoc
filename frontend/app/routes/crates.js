import Ember from 'ember';

export default Ember.Route.extend({
    model(params) {
        alert(this.get('store').peekRecord('crate', params.crate_name));
        return this.get('store').peekRecord('crate', params.crate_name);
    }
});
