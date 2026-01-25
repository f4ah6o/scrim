クロスプラットフォーム対応キーボード駆動型デスクトップナビゲーションシステム（Homerow/Vimium型）の実現可能性およびアーキテクチャ設計に関する包括的研究報告書
1. 序論：HCIにおけるパラダイムシフトと本研究の目的
1.1 研究の背景とHomerow/Vimiumの概念
現代のグラフィカルユーザーインターフェース（GUI）は、マウスやトラックパッドといったポインティングデバイスによる操作を前提に設計されている。しかし、プログラマやパワーユーザーの間では、ホームポジションから手を離すことによるコンテキストスイッチのコスト（Fittsの法則における移動時間の増大）を最小化するため、キーボードのみで操作を完結させる需要が根強く存在する。
この需要に応える形で登場したのが、Google Chrome拡張機能である「Vimium」や、macOS用のユーティリティ「Homerow」である。これらのアプリケーションは「ヒント（Hinting）」と呼ばれる手法を採用している。これは、画面上のインタラクティブな要素（ボタン、リンク、入力フォームなど）を動的に解析し、それぞれに短い文字列（ラベル）をオーバーレイ表示し、ユーザーがその文字列をタイプすることでクリック操作をシミュレートするものである。
1.2 本報告書の目的とスコープ
本報告書は、ユーザーからの要求に基づき、HomerowやVimiumと同様の機能を持つデスクトップアプリケーションを、Windows、macOS、およびUbuntu（Linux）のクロスプラットフォーム環境において、オープンソースソフトウェア（OSS）として構築するための技術的実現可能性、アーキテクチャ設計、および実装戦略を包括的に分析するものである。
特に、以下の技術的課題に焦点を当てる：
 * UI情報の取得（Introspection）: 各OSの異なるアクセシビリティAPI（Accessibility API, UI Automation, AT-SPI2）を用いて、画面上の要素ツリーをどのように取得・解析するか。
 * 視覚情報のオーバーレイ（Visualization）: クリックスルー（操作透過）可能な透明ウィンドウを各OSのウィンドウシステム（Quartz, Win32, Wayland/X11）上でどのように描画するか。
 * 入力操作の制御（Interaction）: グローバルホットキーのフックと、各要素に対するクリックイベントの注入（Injection）をどのように安全かつ高速に行うか。
開発言語としては、システムレベルの低レイヤー操作における安全性とパフォーマンス、およびFFI（Foreign Function Interface）の親和性からRustを採用し、GUIフレームワークとして軽量かつクロスプラットフォーム対応が進むTauri v2を用いる構成を前提として論じる。
2. クロスプラットフォーム・アクセシビリティAPIのランドスケープ分析
Homerow型アプリの核となる技術は、OSが提供するアクセシビリティAPI（Assistive Technology API）を利用して、他プロセスのGUI構造を読み取る「イントロスペクション（内省）」機能である。これらは本来スクリーンリーダー（視覚障害者支援ツール）のために設計されたものであるが、UI自動化にも転用可能である。しかし、各OSの実装は歴史的経緯により完全に分断されており、統一的な抽象化層の設計が最大の課題となる。
2.1 OSごとのAPIアーキテクチャ比較
各プラットフォームにおけるアクセシビリティAPIの技術的特性を以下の表にまとめる。
| 特性 | macOS | Windows | Linux (Ubuntu/GNOME) |
|---|---|---|---|
| コアAPI | Accessibility API (AX) | UI Automation (UIA) | AT-SPI2 (Assistive Technology Service Provider Interface) |
| 基盤技術 | Core Foundation / C言語 | COM (Component Object Model) | D-Bus (Desktop Bus) IPC |
| 座標系 | 左下原点 (Cartesian) | 左上原点 (Physical Pixels) | 左上原点 (Logical Pixels) |
| セキュリティ | TCC (Privacy & Security) | UIA Access権限 (比較的緩やか) | Waylandによる厳格な分離 |
| ツリー構造 | AXUIElement ノード | AutomationElement ノード | Accessible D-Busオブジェクト |
| Rustバインディング | macos-accessibility-client  | uiautomation-rs, windows  | atspi (zbusベース)  |
2.2 macOS: Accessibility API (AX) の深層
macOSにおけるUI操作は、ApplicationServicesフレームワークの一部であるAccessibility APIによって提供される。これはC言語ベースのAPIであり、UI要素はAXUIElementRefという不透明な参照として扱われる。
2.2.1 信頼されたプロセス（Trusted Process）の壁
macOSのセキュリティモデル（TCC）において、他アプリのUIを操作する権限は非常に強力であるため、アプリケーションは「信頼されたプロセス」である必要がある。
 * 権限の確認: AXIsProcessTrusted()関数を用いて、現在のプロセスが権限を持っているかを確認する。
 * ユーザー承認の必須化: 権限がない場合、ユーザーに対して「システム設定」→「プライバシーとセキュリティ」→「アクセシビリティ」への手動追加を促すプロンプトを表示する必要がある（AXIsProcessTrustedWithOptionsを使用）。
 * App Storeの制限: サンドボックス化されたアプリ（Mac App Storeで配布されるアプリ）は、自身のウィンドウ以外のUI要素にアクセスできないという制約がある。したがって、本OSSプロジェクトはApp Store外での配布（Notarized Binary）を前提とする必要がある。
2.2.2 パフォーマンスと属性取得の最適化
AXUIElementを用いたツリー探索は、プロセス間通信（IPC）を伴うためコストが高い。単純な再帰探索を行うと、数千の要素を持つWebページ（Chromeなど）では数百ミリ秒の遅延が発生し、UXを損なう。
 * 最適化戦略: AXUIElementCopyAttribu[span_1](start_span)[span_1](end_span)teValueで一つずつ属性（位置、サイズ、役割）を取得するのではなく、AXUIElementCopyMultipleAttributeValuesを使用して一度のIPCコールで複数の属性を一括取得することが必須となる。これにより、オーバーヘッドを劇的に削減できる。
2.3 Windows: UI Automation (UIA) の深層
Windowsは、かつてのMSAA (Microsoft Active Accessibility) から、よりモダンで堅牢なUI Automation (UIA) へと移行している。UIAはCOM (Component Object Model) を基盤としており、Rustのwindows-rsクレートとの親和性が高い。
2.3.1 ビュー（View）とフィルタリング
UIAの最大の特徴は、UIツリーに対してOSレベルでのフィルタリング機能「ビュー」を提供している点である。
 * Raw View: すべての要素を含む完全なツリー。レイアウト用のコンテナや装飾なども含まれ、探索には非効率である。
 * Control View: ユーザーが対話可能な要素（ボタン、リスト、入力欄など）のみを抽出したビュー。
 * Content View: データを持つ要素のみのビュー。
Homerow型アプリの開発においては、Control Viewを利用することで、アプリケーション側でのフィルタリング処理を最小限に抑え、高速な要素抽出が可能となる。Rust側の実装では、IUIAutomation::CreateTreeWalkerを用いてControlViewWalkerを生成し、これを用いてツリーを走査するアーキテクチャが推奨される。
2.3.2 DPIスケーリングの落とし穴
WindowsのUIAが返す座標（Bounding Rectangle）は物理ピクセルである。一方、TauriなどのWebベースのレンダリングエンジンは論理ピクセルを使用する。高解像度ディスプレイ（HiDPI）環境下で、OSのスケーリング設定（例: 150%）が適用されている場合、物理座標を論理座標に変換するロジック（Physical / ScaleFactor）を実装しない限り、ヒントの表示位置がズレるという致命的な問題が発生する。
2.4 Linux: AT-SPI2とWaylandの分断
Ubuntu（GNOME）を含む現代のLinuxデスクトップ環境は、表示サーバープロトコルがX11からWaylandへと移行中であり、これがアクセシビリティの実装における最大の障壁となっている。
2.4.1 AT-SPI2とD-Busの構造的課題
Linuxのアクセシビリティ標準であるAT-SPI2は、D-Busプロトコル上で動作する。アプリケーション（サーバー）は自身のUIツリーをD-Busオブジェクトとして公開し、支援ツール（クライアント）はD-Busメソッドを呼び出して情報を取得する。
 * レイテンシの問題: D-Busは高頻度な大量通信には不向きである。数千のノードを持つツリーを走査するために数千回のD-Busコールを行うと、秒単位のラグが発生する可能性がある。
 * キャッシング戦略: Rust製のスクリーンリーダー「Odilia」の実装を参考にすると、Cache:Addシグナルを監視し、アプリケーション側からプッシュされるツリー情報の更新差分をローカルにキャッシュするアーキテクチャが必要となる。これにより、リアルタイムでのツリー走査が可能となる。
2.4.2 Waylandのセキュリティモデル
X11では、すべてのアプリケーションがXサーバーを介して他アプリのウィンドウ情報を自由に取得できたが、Waylandではセキュリティ上の理由からクライアント間の分離が徹底されている。
 * 座標の取得: Waylandコンポジタは、クライアントに対して他ウィンドウの絶対座標や存在を隠蔽する。しかし、AT-SPI2はD-Bus経由で動作するため、Waylandのプロトコル外でUI情報（論理的な座標含む）を取得可能である。ただし、取得した座標が実際のスクリーン上のどこに対応するかは、コンポジタの実装やウィンドウの状態に依存するため、複雑なマッピングが必要となる。
3. 実装アーキテクチャ設計：Rust + Tauri v2
本プロジェクトでは、パフォーマンスとクロスプラットフォーム性のバランスから、コアロジックをRust、UIレンダリングをTauri v2で行う「ハイブリッド・アーキテクチャ」を提案する。
3.1 システム構成図
 * Backend (Rust Process):
   * Scanner: 各OSのアクセシビリティAPIを叩き、UIツリーを取得・解析するモジュール。
   * Mapper: 取得したUI要素にヒント文字列（例: "AS", "DF"）を割り当て、座標変換を行う。
   * Input Hook: グローバルキーボードイベントをフックし、ヒント入力か通常入力かを判定する。
   * Injector: クリックイベントをOSレベルでシミュレートする。
 * Frontend (Webview Process):
   * Overlay UI: 全画面透明ウィンドウ上に、Backendから送られたJSONデータに基づきヒント（HTML/CSS）を描画する。
3.2 Rustによるアクセシビリティクライアントの実装詳細
3.2.1 抽象化レイヤー（Trait定義）
クロスプラットフォーム対応のため、以下のようなTrait（インターフェース）を定義し、各OSごとの実装を隠蔽する。
pub struct UIElement {
    pub id: String,
    pub role: String,
    pub rect: Rect, // x, y, width, height
    pub actions: Vec<String>, // "Click", "Focus"など
}

pub trait AccessibilityClient {
    fn get_active_window(&self) -> Result<UIElement>;
    fn traverse_tree(&self, root: &UIElement) -> Vec<UIElement>;
    fn perform_action(&self, element_id: &str, action: &str) -> Result<()>;
}

3.2.2 Linux向けAT-SPI2実装（atspiクレートの活用）
Linux版の実装では、Odiliaプロジェクトが開発したatspiクレートが極めて重要となる。これは非同期D-Busラッパーであるzbus上に構築されており、Rustのasync/await構文を用いてノンブロッキングにUIツリーを取得できる。
 * 依存関係: atspi, zbus
 * 実装ポイント: Event::WindowやEvent::Objectシグナルを購読し、ウィンドウのアクティブ化やUIの変化を検知してツリーを再スキャンするトリガーとする。
3.2.3 Windows向けUIA実装（uiautomation-rsの活用）
Windows版ではuiautomation-rsまたはMicrosoft公式のwindowsクレートを使用する。
 * スレッドモデル: UIAはCOMベースであるため、RustのスレッドがCOMアパートメント（MTA/STA）として正しく初期化されている必要がある（CoInitializeEx）。Tauriのメインスレッドとは別のスレッドでこれを行う設計が必要である。
3.2.4 macOS向けAX実装
macOS版はaccessibility-sysを用いてC APIを呼び出す。RustのFFI機能により、CoreFoundationの型（CFArrayRef, AXUIElementRef）をRustの構造体に安全にラップして扱う。
4. 視覚レイヤー：Tauri v2によるオーバーレイとクリックスルー問題
Homerowのようなアプリでは、画面全体を覆う透明なウィンドウを表示し、そこにヒントを描画する必要がある。しかし、単に透明なだけでは、ユーザーがマウスで下のウィンドウを操作しようとした際に邪魔になってしまう。ここで「クリックスルー（Click-Through）」あるいは「Input Passthrough」と呼ばれる技術的課題が発生する。
4.1 クリックスルーの技術的要件
オーバーレイウィンドウは以下の挙動を満たす必要がある：
 * 視覚: 常に最前面（Always on Top）にあり、背景は透明。
 * 入力: ヒントが表示されている「矩形領域」以外の場所へのマウスクリックは、下のウィンドウに透過（Pass-through）させる。
4.2 各OSにおける実装戦略
4.2.1 Windows: WS_EX_TRANSPARENT
Windowsにおいては、Win32 APIの拡張ウィンドウスタイルWS_EX_TRANSPARENTを利用する。
 * Tauriでの実装: Tauriのウィンドウハンドル（HWND）を取得し、Rust側でSetWindowLongPtr関数を用いてこのスタイルを適用する。
 * 注意点: WS_EX_TRANSPARENTを設定すると、そのウィンドウはマウスクリックを一切受け付けなくなる。ヒント自体をクリック可能にする場合、このフラグを動的に切り替えるか、あるいはマウスイベントをローレベルでフックして自前で処理する必要がある。
4.2.2 macOS: ignoresMouseEvents
macOSのCocoa APIでは、NSWindowのignoresMouseEventsプロパティをYESに設定することで、ウィンドウ全体をクリックスルーにできる。
 * 動的制御: ヒント表示中もユーザーがマウス操作を行えるようにするには、マウスカーソルがヒントの直上にある時だけignoresMouseEventsをNOにし、それ以外はYESにするような細かい制御が必要となる場合がある。
4.2.3 Linux (Wayland/GTK4): Input Regions
Linux、特にWayland環境下での実装が最も困難である。Waylandではクライアントが自身のウィンドウの形状を自由に定義できない制約がある。
 * GTK4のInput Region: Tauri v2が使用するGTK4では、gdk_surface_set_input_region関数を使用して、ウィンドウ内の「入力を受け付ける領域」を明示的に定義できる。
 * 実装ロジック:
   * Rustバックエンドでヒントの座標リストを生成。
   * その座標リスト（矩形の集合）をCairo Regionとして生成。
   * Tauriのウィンドウ（GtkWindow）からNativeサーフェスを取得し、set_input_regionを適用。
     これによって、ヒントがある場所だけがクリック可能になり、それ以外は透明かつ操作透過となる完璧なオーバーレイが実現できる。
4.3 Waylandにおけるレイヤーシェル（Layer Shell）
Waylandコンポジタ（Sway, Hyprland, KDE Plasmaなど）上で、通常のウィンドウではなく「オーバーレイ」として振る舞うためには、Wayland Layer Shellプロトコル（wlr-layer-shell）の使用が必須である。
 * Rustクレート: gtk4-layer-shellクレートを使用することで、GTK4ウィンドウをLayer Shellサーフェスとして昇格させることができる。
 * GNOMEの課題: UbuntuのデフォルトであるGNOME Shellは、歴史的にLayer Shellプロトコルをネイティブサポートしていない（独自のAPIを使用）。そのため、GNOME環境ではXWayland経由でX11アプリとして動作させるか、GNOME Shell Extensionを併用する回避策が必要となる可能性がある。
5. 入力・実行レイヤー：イベント注入と操作シミュレーション
ユーザーがヒント（例: "K"）を入力した際、その対象となるボタンをクリックする動作（Action）の実装について詳述する。
5.1 実行戦略A：アクセシビリティアクション（推奨）
可能であれば、マウスカーソルを動かさずに、API経由で直接「クリック」イベントを発火させるべきである。
 * macOS: AXUIElementPerformAction(element, kAXPressAction) 。
 * Windows: IInvokeProvider::Invoke() メソッドを呼び出す。
 * Linux: AT-SPI2のActionインターフェースにあるDoActionメソッドを使用する。
5.2 実行戦略B：入力シミュレーション（フォールバック）
Electron製アプリやWebページなど、アクセシビリティツリーは存在するがアクションが正しく配線されていない場合、物理的なマウスクリックをシミュレートする必要がある。
5.2.1 座標計算とDPI
要素の座標 (x, y, w, h) から中心点 (x + w/2, y + h/2) を計算し、そこへマウスを移動させてクリックを送る。この際、前述のDPIスケーリングの補正が再度重要となる。
5.2.2 Waylandにおける入力注入の壁
Waylandはセキュリティ設計上、アプリケーションがマウスカーソルを強制的に移動させたり、他アプリへ入力を注入することを禁止している。
 * 解決策：uinput: Linuxカーネルレベルの入力サブシステムであるuinputを利用して、仮想マウス・仮想キーボードデバイスを作成する。これにより、Waylandコンポジタの制限を回避してOSレベルで入力を行える。
 * 実装: Rustのmouse-keyboard-inputクレートを使用する。
 * 権限: uinputを使用するには、ユーザーが/dev/uinputへの書き込み権限を持っている必要がある（uinputグループへの追加やudevルールの設定が必要）。これはインストール時のセットアップガイドでユーザーに明示する必要がある重要な要件である。
5.3 グローバルホットキーの監視
アプリを起動するためのホットキー（例: Cmd+Shift+Space）の監視には、Tauriの公式プラグインtauri-plugin-global-shortcutを利用するのが最も安全である。
 * Linuxでの注意: Wayland環境では、グローバルショートカットもセキュリティ制限の対象となる。Tauriプラグインは裏側でxdg-desktop-portalのGlobalShortcutsインターフェースを使用しており、初回起動時にユーザーに対してショートカット登録の許可を求めるダイアログが表示される仕組みになっている。
6. 結論と開発ロードマップ
6.1 結論：OSS化の実現可能性
調査の結果、Homerow/Vimiumライクなクロスプラットフォーム・キーボードナビゲーションアプリのOSS開発は、技術的に十分に可能であるとの結論に至った。
特に、Rustエコシステムの成熟（atspi, uiautomation-rs, accessibility-sysなどのクレート群）により、かつてはC++やObjective-Cで個別に書く必要があった低レイヤー処理を、単一の言語で安全かつ効率的に記述できるようになった点が大きい。また、Tauri v2の登場により、透明ウィンドウやマルチウィンドウの扱いが容易になり、UI部分の開発コストを大幅に削減できる。
ただし、Linux (Wayland) 環境における実装コストは、Windows/macOSに比べて著しく高い。特に、レイヤーシェルによるオーバーレイ表示の実装や、uinputを用いた入力シミュレーションの権限管理など、Linux特有の複雑な事情に対処する必要がある。Ubuntu（GNOME）ユーザーに対しては、これらの制約や設定手順（udevルールの追加など）を明確にドキュメント化することが普及の鍵となる。
6.2 推奨開発ステップ
 * フェーズ1：インスペクタの開発 (Probe)
   * 各OSでアクティブウィンドウのアクセシビリティツリーをダンプするCLIツールをRustで作成し、APIの挙動とパフォーマンスを検証する。特に、ブラウザなどの巨大なツリーに対するフィルタリングロジック（Control View相当）の確立を行う。
 * フェーズ2：オーバーレイの実装 (Visual)
   * Tauri v2を用いて、透明かつクリックスルー可能なウィンドウを表示するプロトタイプを作成する。Linuxにおいてはgtk4-layer-shellを用いたWayland対応を検証する。
 * フェーズ3：コアロジックの統合 (Integration)
   * ツリー構造からヒントラベル（AA, AB,...）を生成するアルゴリズム（最短文字列生成）を実装し、RustバックエンドとTauriフロントエンドをIPCで接続する。
 * フェーズ4：アクションの実装 (Action)
   * OSごとのアクション実行（APIコール vs マウスシミュレーション）を実装する。Linux向けにはuinputデバイスの初期化ロジックを組み込む。
本報告書で示したアーキテクチャに基づき開発を進めることで、既存の商用ツールに匹敵する、あるいはそれを凌駕する柔軟性を持ったOSSプロダクトを実現できると確信する。

