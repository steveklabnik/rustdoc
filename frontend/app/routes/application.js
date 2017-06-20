import Ember from 'ember';

export default Ember.Route.extend({
    beforeModel() {
        return this._populateStore();
    },

    _populateStore() {
        // sigh https://stackoverflow.com/questions/2618959/
        Ember.$.ajaxSetup({
            beforeSend: function (xhr) {
                if (xhr.overrideMimeType) {
                    xhr.overrideMimeType("application/json");
                }
            }
        });
        return Ember.$.getJSON('data.json').then(data => {
            this.store.push(data);
        });
    }
});
