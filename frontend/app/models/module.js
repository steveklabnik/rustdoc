import DS from 'ember-data';

export default DS.Model.extend({
    name: DS.attr(),
    docs: DS.attr(),
    crate: DS.belongsTo('crate')
});
