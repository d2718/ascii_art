/* script.js

ascii_art CGI script frontend JSBS

updated 2022-05-19
*/

// CGI testing echo endpoint.
//const URI = "https://d2718.net/cgi-bin/testor.cgi";

// The actual ASCII art API endpoint.
const URI = "https://d2718.net/cgi-bin/aa_cgi.cgi";

const OUTPUT = document.getElementById("output");
const SUBMIT = document.getElementById("submit_button");
const FONTS  = document.getElementById("font_input");
const SIZES  = document.getElementById("size_input");
const STATUS = {
  "div": document.getElementById("status"),
  "p": document.querySelector("div#status > p"),
};
const ERROR = {
  "div": document.getElementById("error"),
  "p": document.querySelector("div#error > p"),
  "button": document.getElementById("error_dismiss"),
};
const FONTSOURCE = {
  "server": document.getElementById("font_server"),
  "user": document.getElementById("font_user")
};
const FONTFILE = document.getElementById("font_file");

/*
Holds data about server-supplied font metrics.

Will be populated after page load by call to populate_choices().
*/
var CHOICES = new Map();

// Recursively remove all children of a given element.
function clear(elt) {
  while (elt.firstChild) {
    clear(elt.lastChild);
    elt.removeChild(elt.lastChild);
  }
}

// Append the supplied `txt` to the status <div> and set the <div> to visible.
function add_output(txt) {
  const node = document.createTextNode(txt);
  OUTPUT.appendChild(node);
  OUTPUT.appendChild(document.createTextNode("\n"));
  OUTPUT.style.display = "block";
}

/*
Generate a Map with the [key, value] pairs of the supplied object.

This is for translating deserialized JSON data into a Map.
*/
function obj2map(obj) {
  const m = new Map();
  for(const [k, v] of Object.entries(obj)) {
    m.set(k, v);
  }
  
  return m;
}

// Clear any extant status text from the status <div>, set the status text to
// the supplied `text`, and set the <div> to be visible.
function set_status(text) {
  clear(STATUS.p);
  STATUS.p.appendChild(document.createTextNode(text));
  STATUS.div.style.display = "inline-flex";
}

// Hide the status <div>.
function hide_status() {
  STATUS.div.style.display = "none";
}

// Add text to the text in the error <div> and ensure the <div> is visible.
function add_error(txt) {
  const node = document.createTextNode(txt);
  ERROR.p.appendChild(node);
  ERROR.p.appendChild(document.createTextNode("\n"));
  ERROR.div.style.display = "inline-flex";
}

/*
Clear all error text from the error <div> and hide it.

This should be called when the user presses on the error dismissal button
in the error <div>.
*/
function dismiss_error(evt) {
  evt.preventDefault();
  clear(ERROR.p);
  ERROR.div.style.display = "none";
}

/*
Enable/disable the mutually exclusive pair of font source inputs.

The input with a choice of server-provided fonts and the file input
for a user to supply their own font file should not be simultaneously
enabled; this function toggles between the two. It is called by selecting
one of the "font_source" radio buttons.
*/
function toggle_font_source() {
  if(FONTSOURCE.server.checked) {
    FONTFILE.disabled = true;
    FONTS.disabled = false;
  } else {
    FONTS.disabled = true;
    FONTFILE.disabled = false;
  }
}

/*
Submit a request to the server to translate the selected image into ASCII.

Will populate the output <div> on success, or display a (hopefully-informative)
error message otherwise.
*/
function submit(evt) {
  evt.preventDefault();
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
  
  clear(OUTPUT);
  
  /*
  Set the colors of the output <div> to match the color target selected by
  the user (either the defaut light-on-dark or the inverted dark-on-light).
  */
  if(data.get("invert") == "false") {
    OUTPUT.setAttribute("class", "normal-color");
  } else if(data.get("invert") == "true") {
    OUTPUT.setAttribute("class", "inverted-color");
  }
  OUTPUT.style.fontSize = `${data.get("size")}px`;
  
  fetch(URI, request_object)
  .then(r => {
    if(r.status == 200) {
      r.text()
      .then(t => {
        clear(OUTPUT);
        add_output(t);
      })
    } else {
      r.text()
      .then(t => {
        add_error(t);
        add_error(`(Error ${r.status})`);
      })
    }
  })
  .catch(e => { add_error(new String(e))})
  .finally(hide_status);
}

/*
When a font is selected from the drop-down <select> of font families for
which the server has metrics, this function is called to populatethe
choices in the "size" <select> that are available for that font.
*/
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

/*
Called on page load, this function requests from the server a catalog
of the fonts (and sizes) for which it has metric data and thus can use
as rendering targets.

It then sets the choices in the font-family <select> and populates the
size <select> with the sizes for the default font choice.
*/
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
    if(r.status != 200) {
      r.text()
      .then(t => {
        add_error(`Error fetching font data (${r.status}):\n`);
        add_error(t);
      });
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
    add_error(`Error populating font data: ${e}`);
  })
  .finally(hide_status);
}

/*
Adds event listeners to appropriate page elements, and calls populate_choices()
to query the server about its available font data, and populates the
appropriate inputs.
*/
function init() {
  ERROR.button.addEventListener("click", dismiss_error);
  populate_choices();
  SUBMIT.addEventListener("click", submit);
  FONTS.addEventListener("change", populate_sizes);
  for (const rb of document.querySelectorAll('input[name = "font_source"]')) {
    rb.addEventListener("click", toggle_font_source);
  }
}

init();