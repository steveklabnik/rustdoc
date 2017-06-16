import DS from 'ember-data';

export default DS.Model.extend({
    name: DS.attr(),
    docs: DS.attr(),
    modules: DS.hasMany('modules')
});
