import Component, {tracked} from "@glimmer/component";

export default class Frontend extends Component {
  @tracked results;

  constructor(options) {
    super(options);
    this.getFeed();
  };

  async getFeed(){
    let data = await fetch('data.json');
    this.results = await data.json();
  };
}

