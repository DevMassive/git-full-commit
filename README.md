# おい、お前。フルコミットしてるか？

これは常にフルコミットする漢の Git Commit CLI ツール。

<img alt="Screenshot" src="https://github.com/user-attachments/assets/dddd9593-3f6c-41b1-9c1a-d7f1be39e732" width="480"> 

```
% git add -A
% git reset -p
% git commit [--amend]
```

を直感的なUIで行う。
それ以外の操作はあんまりできないんだけど、漢なら常にフルコミットだから問題がない。

# インストール方法

## バイナリをダウンロード

[Releases](https://github.com/DevMassive/git-full-commit/releases) 展開後、PATHの通った場所に置く。

## ソースコードからビルド

Cargoが必要。

```
cargo install --git https://github.com/DevMassive/git-full-commit
```

# 使い方

git管理しているディレクトリ内で実行。

```
% git-full-commit
```

すると、すべての変更がStageされる（すでにStageされた変更がない場合のみ）。

## Diff操作

- ↑↓: ファイル選択
- j/k: Diff内カーソル移動
- ←→: Diffの水平スクロール
- Space/b: ページスクロール
- Ctrl+d/Ctrl+u: 半ページスクロール
- ENTER: ファイル/ハンクのステージをやめる
- 1: 選択行のステージをやめる
- !: ファイル変更を完全に消す
- i: ファイルを.gitignoreに追加
- R: 改めてすべての変更をStageする
- Ctrl+cとかqとか: 終了

## Undo/Redo

漢なら使わないと思うがCommit以外の操作はUndo/Redoができる。

- u/r: undo/redo

## Commit操作

ファイルリストの下にある入力欄にコミットメッセージをいれてENTERを押せばフルコミット完了。
- もしまだフルコミットできてないなら、改めてすべての変更がStageされて続行
- 入力欄でTABを押せばAmendに切り替わる

# 関連プロジェクト

- [tig](https://github.com/jonas/tig)
- [Lazygit](https://github.com/jesseduffield/lazygit)
- [GitUI](https://github.com/gitui-org/gitui)

# 動作環境

俺のマック

# コントリビューション

コントリビューションは大歓迎！
issueの起票やpull requestをお気軽に！
AIでもいいよ！

# License

This project is licensed under the MIT License.

