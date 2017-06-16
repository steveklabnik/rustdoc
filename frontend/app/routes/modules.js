import Ember from 'ember';

export default Ember.Route.extend({
    model(params) {
        // params.crate_name;
        // params.path;
        return this.get('store').peekRecord('module', params.crate_name + "::" + params.path);
    }
});
