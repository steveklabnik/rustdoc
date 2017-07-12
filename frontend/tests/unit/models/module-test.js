import { moduleForModel, test } from 'ember-qunit';

moduleForModel('module', 'Unit | Model | module', {
  // Specify the other units that are required for this test.
  needs: ['model:crate']
});

test('it exists', function(assert) {
  let model = this.subject();
  assert.ok(!!model);
});
