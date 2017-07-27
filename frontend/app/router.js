import Ember from 'ember';
import config from './config/environment';

const Router = Ember.Router.extend({
  location: config.locationType,
  rootURL: config.rootURL
});

Router.map(function() {
  this.route('crates', { path: ':crate_name' });
  this.route('modules', { path: '*path' });
});

export default Router;
