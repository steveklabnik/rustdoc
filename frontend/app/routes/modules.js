import Ember from 'ember';

export default Ember.Route.extend({
  // serialize is called whenever you want to turn this route's model into a
  // URL. For this reason, we need to turn `::`s into `/`s when producing URLs.
  serialize(mod) {
    return { path: mod.get('id').replace(/::/g, '/') };
  },

  // To find the model from the URL, we need to convert /s to ::s, since the
  // URLs have /s but the IDs have ::s.
  model(params) {
    return this.store.peekRecord('module', params.path.replace(/\//g, '::'));
  }
});
