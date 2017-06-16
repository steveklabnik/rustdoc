import DS from 'ember-data';

export default DS.Model.extend({
    docs: DS.attr(),
    modules: DS.hasMany('modules')
});
