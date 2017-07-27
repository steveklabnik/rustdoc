import DS from 'ember-data';

export default DS.Model.extend({
    name: DS.attr(),
    docs: DS.attr(),
    crate: DS.belongsTo('crate'),
    child_modules: DS.hasMany('modules', { inverse: 'parent_module'}),
    parent_module: DS.belongsTo('module', { inverse: 'child_modules'}),
});
