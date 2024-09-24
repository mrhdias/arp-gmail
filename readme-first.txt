* How to use this plugin:

First edit the configuration file "config.json" and set with your cardentials.
https://myaccount.google.com/apppasswords

* How to use the template in your Tera application templates?

let mut tera = Tera::new("examples/templates/**/*.html").unwrap();

cd examples/templates
ln -s ../../plugins ./plugins

Insert this in your template:
-----------------------------
<div style="margin: 20px 0;">
    {% include "plugins/arp-gmail/templates/form.html" %}
</div>