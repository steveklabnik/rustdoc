import Ember from 'ember';

export default Ember.Route.extend({
    model({ crate_name, path }) {
        return this.get('store').peekRecord('module', `${crate_name}::${path}`);
    }
});
