/* script.js

ascii_art CGI script frontend JSBS
*/
//const URI = "https://d2718.net/cgi-bin/testor.cgi";
const URI = "https://d2718.net/cgi-bin/aa_cgi.cgi";

const OUTPUT = document.getElementById("output");
const SUBMIT = document.getElementById("submit_button");
const FONTS  = document.getElementById("font_input");
const SIZES  = document.getElementById("size_input");
const STATUS = document.getElementById("status");
const ERROR  = document.getElementById("error");

var CHOICES = new Map();

function clear(elt) {
  while (elt.firstChild) {
    clear(elt.lastChild);
    elt.removeChild(elt.lastChild);
  }
}

function add_output(txt) {
  const node = document.createTextNode(txt);
  OUTPUT.appendChild(node);
  OUTPUT.appendChild(document.createTextNode("\n"));
}

function obj2map(obj) {
  const m = new Map();
  for(const [k, v] of Object.entries(obj)) {
    console.log(k);
    console.log(v);
    m.set(k, v);
  }
  
  return m;
}

function set_status(text) {
  clear(STATUS);
  STATUS.appendChild(document.createTextNode(text));
  STATUS.style.display = "inline-block";
}

function hide_status() {
  STATUS.style.display = "none";
}

function submit() {
  const form = document.getElementById("the_form");
  const data = new FormData(form);
  const heads = new Headers();
  heads.set("aa-action", "render");
  
  let request_object = {
    method: "POST",
    mode: "cors",
    headers: heads,
    body: data,
  };
  
  set_status("rendering...");
  
  console.log(request_object);
  
  clear(OUTPUT);
  add_output("sent request...");
  
  fetch(URI, request_object)
  .then(r => {
    add_output(`response status: ${r.status}`);
    if(r.status == 200) {
      r.text()
      .then(t => {
        clear(OUTPUT);
        add_output(t);
      })
    } else {
      r.text()
      .then(t => {
        add_output(t);
      })
    }
  })
  .catch(console.log)
  .finally(hide_status);
}

function populate_sizes() {
  const font_name = FONTS.value;
  const sizes = CHOICES.get(font_name);
  
  clear(SIZES);
  for(s of sizes) {
    let opt = document.createElement("option");
    opt.setAttribute("value", s);
    opt.appendChild(document.createTextNode(s));
    SIZES.appendChild(opt);
  }
}


function populate_choices() {
  const heads = new Headers();
  heads.set("aa-action", "list");
  
  let request_object = {
    method: "GET",
    mode: "cors",
    headers: heads,
  }
  
  set_status("Fetching font data...");
  
  fetch(URI, request_object)
  .then(r => {
    console.log(r);
    if(r.status != 200) {
      add_output(`response status is ${r.status}`);
      r.text()
      .then(add_output);
    } else {
      r.json()
      .then(m => {
        CHOICES = obj2map(m);

        const font_names = new Array();
        for(const [k, v] of CHOICES) {
          font_names.push(k);
          v.sort((a, b) => a - b);
        }
        font_names.sort();
        clear(FONTS);
        for(name of font_names) {
          const opt = document.createElement("option");
          opt.setAttribute("value", name);
          opt.appendChild(document.createTextNode(name));
          FONTS.appendChild(opt);
        }
        
        populate_sizes();
      })
    }
  })
  .catch(e => {
    add_output(`caught error:\n${e}`);
  })
  .finally(hide_status);
}

function init() {
  populate_choices();
  SUBMIT.addEventListener("click", submit);
  FONTS.addEventListener("change", populate_sizes);
}

init();