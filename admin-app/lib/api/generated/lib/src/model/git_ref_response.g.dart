// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'git_ref_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$GitRefResponse extends GitRefResponse {
  @override
  final String name;
  @override
  final String ref;
  @override
  final String sha;

  factory _$GitRefResponse([void Function(GitRefResponseBuilder)? updates]) =>
      (GitRefResponseBuilder()..update(updates))._build();

  _$GitRefResponse._({required this.name, required this.ref, required this.sha})
    : super._();
  @override
  GitRefResponse rebuild(void Function(GitRefResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  GitRefResponseBuilder toBuilder() => GitRefResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is GitRefResponse &&
        name == other.name &&
        ref == other.ref &&
        sha == other.sha;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, ref.hashCode);
    _$hash = $jc(_$hash, sha.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'GitRefResponse')
          ..add('name', name)
          ..add('ref', ref)
          ..add('sha', sha))
        .toString();
  }
}

class GitRefResponseBuilder
    implements Builder<GitRefResponse, GitRefResponseBuilder> {
  _$GitRefResponse? _$v;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _ref;
  String? get ref => _$this._ref;
  set ref(String? ref) => _$this._ref = ref;

  String? _sha;
  String? get sha => _$this._sha;
  set sha(String? sha) => _$this._sha = sha;

  GitRefResponseBuilder() {
    GitRefResponse._defaults(this);
  }

  GitRefResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _name = $v.name;
      _ref = $v.ref;
      _sha = $v.sha;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(GitRefResponse other) {
    _$v = other as _$GitRefResponse;
  }

  @override
  void update(void Function(GitRefResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  GitRefResponse build() => _build();

  _$GitRefResponse _build() {
    final _$result =
        _$v ??
        _$GitRefResponse._(
          name: BuiltValueNullFieldError.checkNotNull(
            name,
            r'GitRefResponse',
            'name',
          ),
          ref: BuiltValueNullFieldError.checkNotNull(
            ref,
            r'GitRefResponse',
            'ref',
          ),
          sha: BuiltValueNullFieldError.checkNotNull(
            sha,
            r'GitRefResponse',
            'sha',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
