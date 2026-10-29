// Matrix - Kod Yağmuru: GNOME Shell eklentisi.
//
// Bilgisayar ayarlanan süre kadar boşta kalınca eklentinin içindeki çizim
// programını başlatır; program her ekranda tam ekran çizer ve bir tuş, tıklama
// veya fare hareketiyle kendini kapatır. GNOME ekranı karartmaya veya
// kilitlemeye başlayınca program kapatılır. Kilit, parola ve ekranın
// kapanması GNOME'un işidir; eklenti bunlara dokunmaz.
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {InhibitFlags} from 'resource:///org/gnome/shell/misc/gnomeSession.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

/** Programın kapatılırken aldığı sinyal (SIGTERM). */
const SIGTERM = 15;

export default class MatrixExtension extends Extension {
    enable() {
        this._settings = this.getSettings();
        this._idleMonitor = global.backend.get_core_idle_monitor();
        this._idleWatch = 0;
        // Çalışan programın GNOME tarafındaki bağlantısı; program yoksa boştur.
        this._client = null;

        this._watchIdle();
        this._settings.connectObject('changed::start-delay', () => this._watchIdle(), this);
        global.display.connectObject('window-created', (_display, window) => this._adopt(window), this);
        // GNOME ekranı karartmaya veya kilitlemeye başladı: çizim durur.
        Main.screenShield.connectObject('active-changed', () => this._stop(), this);
    }

    disable() {
        this._stop();
        this._idleMonitor.remove_watch(this._idleWatch);
        this._settings.disconnectObject(this);
        global.display.disconnectObject(this);
        Main.screenShield.disconnectObject(this);
        this._settings = null;
        this._idleMonitor = null;
    }

    /** Boşta kalma süresini ayardan okur ve bekleyişi yeniden kurar. */
    _watchIdle() {
        if (this._idleWatch)
            this._idleMonitor.remove_watch(this._idleWatch);
        const minutes = this._settings.get_uint('start-delay');
        this._idleWatch = this._idleMonitor.add_idle_watch(minutes * 60 * 1000, () => this._start());
    }

    /** Ekran koruyucuyu başlatır. Video izlenirken olduğu gibi bir uygulama
     * boşta kalmayı engellediyse veya ekran zaten kilitliyse başlamaz. */
    async _start() {
        if (this._client || Main.screenShield.active || Main.sessionMode.isLocked)
            return;
        // Beklerken eklenti kapatılmış veya program başlamış olabilir.
        if (await this._idleInhibited() || this._client || !this._settings)
            return;
        try {
            // Program, GNOME'un açtığı özel bir Wayland bağlantısıyla başlar;
            // böylece pencereleri kesin olarak tanınır.
            const program = GLib.build_filenamev([this.path, 'matrixcoderain']);
            const launcher = new Gio.SubprocessLauncher({flags: Gio.SubprocessFlags.NONE});
            const client = Meta.WaylandClient.new_subprocess(global.context, launcher, [program, '/s']);
            this._client = client;
            client.get_subprocess().wait_async(null, () => {
                if (this._client === client)
                    this._client = null;
            });
        } catch (e) {
            console.error(`Matrix - Kod Yağmuru başlatılamadı: ${e.message}`);
        }
    }

    /** Bir uygulama boşta kalmayı engelliyor mu? Oturum yöneticisine
     * ulaşılamazsa engel yok sayılır; ekran koruyucu yine çalışır. */
    async _idleInhibited() {
        try {
            const reply = await Gio.DBus.session.call(
                'org.gnome.SessionManager', '/org/gnome/SessionManager',
                'org.gnome.SessionManager', 'IsInhibited',
                new GLib.Variant('(u)', [InhibitFlags.IDLE]), new GLib.VariantType('(b)'),
                Gio.DBusCallFlags.NONE, -1, null);
            return reply.deepUnpack()[0];
        } catch (e) {
            console.warn(`Matrix - Kod Yağmuru: boşta kalma engeli okunamadı: ${e.message}`);
            return false;
        }
    }

    /** Programın açtığı pencereyi pencere listesinden gizler, en üste alır ve
     * etkinleştirir; böylece ilk tuş ve fare hareketi programa gider. */
    _adopt(window) {
        if (!this._client?.owns_window(window))
            return;
        window.hide_from_window_list();
        window.make_above();
        const shownId = window.connect('shown', () => {
            window.disconnect(shownId);
            Main.activateWindow(window);
        });
    }

    /** Çalışan programı kapatır. */
    _stop() {
        this._client?.get_subprocess().send_signal(SIGTERM);
        this._client = null;
    }
}
