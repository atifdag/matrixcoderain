// Matrix - Kod Yağmuru: GNOME 42–44 için ayar penceresi; tek ayar, başlama
// süresi. ../prefs.js ile aynı ayarı gösterir. libadwaita 1.4 öncesinde
// sayı satırı (SpinRow) olmadığı için sayı kutusu satırın sağına konur.
'use strict';

const {Adw, Gio, Gtk} = imports.gi;

const ExtensionUtils = imports.misc.extensionUtils;

/** GNOME ayar penceresini açarken önce bu işlevi çağırır; hazırlık gerekmez. */
function init() {
}

function fillPreferencesWindow(window) {
    const spin = new Gtk.SpinButton({
        adjustment: new Gtk.Adjustment({lower: 1, upper: 120, step_increment: 1, page_increment: 5}),
        valign: Gtk.Align.CENTER,
    });
    ExtensionUtils.getSettings().bind('start-delay', spin, 'value', Gio.SettingsBindFlags.DEFAULT);

    const row = new Adw.ActionRow({
        title: 'Başlama süresi (dakika)',
        subtitle: 'Bilgisayar bu kadar boşta kalınca ekran koruyucu başlar. ' +
            'Ayarlar › Güç bölümündeki ekranı karartma süresinden kısa olmalıdır.',
        activatable_widget: spin,
    });
    row.add_suffix(spin);

    const group = new Adw.PreferencesGroup();
    group.add(row);
    const page = new Adw.PreferencesPage();
    page.add(group);
    window.add(page);
}
