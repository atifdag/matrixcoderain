// Matrix - Kod Yağmuru eklentisinin ayar penceresi: tek ayar, başlama süresi.
import Adw from 'gi://Adw';
import Gio from 'gi://Gio';
import Gtk from 'gi://Gtk';

import {ExtensionPreferences} from 'resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js';

export default class MatrixPreferences extends ExtensionPreferences {
    fillPreferencesWindow(window) {
        const row = new Adw.SpinRow({
            title: 'Başlama süresi (dakika)',
            subtitle: 'Bilgisayar bu kadar boşta kalınca ekran koruyucu başlar. ' +
                'Ayarlar › Güç bölümündeki ekranı karartma süresinden kısa olmalıdır.',
            adjustment: new Gtk.Adjustment({lower: 1, upper: 120, step_increment: 1, page_increment: 5}),
        });
        this.getSettings().bind('start-delay', row, 'value', Gio.SettingsBindFlags.DEFAULT);

        const group = new Adw.PreferencesGroup();
        group.add(row);
        const page = new Adw.PreferencesPage();
        page.add(group);
        window.add(page);
    }
}
