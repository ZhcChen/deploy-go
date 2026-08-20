// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'application_config_diff_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ApplicationConfigDiffResponse extends ApplicationConfigDiffResponse {
  @override
  final bool changed;
  @override
  final String compareContent;
  @override
  final int? compareVersion;
  @override
  final String currentContent;
  @override
  final int currentVersion;
  @override
  final String fileId;
  @override
  final bool sensitive;

  factory _$ApplicationConfigDiffResponse([
    void Function(ApplicationConfigDiffResponseBuilder)? updates,
  ]) => (ApplicationConfigDiffResponseBuilder()..update(updates))._build();

  _$ApplicationConfigDiffResponse._({
    required this.changed,
    required this.compareContent,
    this.compareVersion,
    required this.currentContent,
    required this.currentVersion,
    required this.fileId,
    required this.sensitive,
  }) : super._();
  @override
  ApplicationConfigDiffResponse rebuild(
    void Function(ApplicationConfigDiffResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ApplicationConfigDiffResponseBuilder toBuilder() =>
      ApplicationConfigDiffResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ApplicationConfigDiffResponse &&
        changed == other.changed &&
        compareContent == other.compareContent &&
        compareVersion == other.compareVersion &&
        currentContent == other.currentContent &&
        currentVersion == other.currentVersion &&
        fileId == other.fileId &&
        sensitive == other.sensitive;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, changed.hashCode);
    _$hash = $jc(_$hash, compareContent.hashCode);
    _$hash = $jc(_$hash, compareVersion.hashCode);
    _$hash = $jc(_$hash, currentContent.hashCode);
    _$hash = $jc(_$hash, currentVersion.hashCode);
    _$hash = $jc(_$hash, fileId.hashCode);
    _$hash = $jc(_$hash, sensitive.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ApplicationConfigDiffResponse')
          ..add('changed', changed)
          ..add('compareContent', compareContent)
          ..add('compareVersion', compareVersion)
          ..add('currentContent', currentContent)
          ..add('currentVersion', currentVersion)
          ..add('fileId', fileId)
          ..add('sensitive', sensitive))
        .toString();
  }
}

class ApplicationConfigDiffResponseBuilder
    implements
        Builder<
          ApplicationConfigDiffResponse,
          ApplicationConfigDiffResponseBuilder
        > {
  _$ApplicationConfigDiffResponse? _$v;

  bool? _changed;
  bool? get changed => _$this._changed;
  set changed(bool? changed) => _$this._changed = changed;

  String? _compareContent;
  String? get compareContent => _$this._compareContent;
  set compareContent(String? compareContent) =>
      _$this._compareContent = compareContent;

  int? _compareVersion;
  int? get compareVersion => _$this._compareVersion;
  set compareVersion(int? compareVersion) =>
      _$this._compareVersion = compareVersion;

  String? _currentContent;
  String? get currentContent => _$this._currentContent;
  set currentContent(String? currentContent) =>
      _$this._currentContent = currentContent;

  int? _currentVersion;
  int? get currentVersion => _$this._currentVersion;
  set currentVersion(int? currentVersion) =>
      _$this._currentVersion = currentVersion;

  String? _fileId;
  String? get fileId => _$this._fileId;
  set fileId(String? fileId) => _$this._fileId = fileId;

  bool? _sensitive;
  bool? get sensitive => _$this._sensitive;
  set sensitive(bool? sensitive) => _$this._sensitive = sensitive;

  ApplicationConfigDiffResponseBuilder() {
    ApplicationConfigDiffResponse._defaults(this);
  }

  ApplicationConfigDiffResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _changed = $v.changed;
      _compareContent = $v.compareContent;
      _compareVersion = $v.compareVersion;
      _currentContent = $v.currentContent;
      _currentVersion = $v.currentVersion;
      _fileId = $v.fileId;
      _sensitive = $v.sensitive;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ApplicationConfigDiffResponse other) {
    _$v = other as _$ApplicationConfigDiffResponse;
  }

  @override
  void update(void Function(ApplicationConfigDiffResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ApplicationConfigDiffResponse build() => _build();

  _$ApplicationConfigDiffResponse _build() {
    final _$result =
        _$v ??
        _$ApplicationConfigDiffResponse._(
          changed: BuiltValueNullFieldError.checkNotNull(
            changed,
            r'ApplicationConfigDiffResponse',
            'changed',
          ),
          compareContent: BuiltValueNullFieldError.checkNotNull(
            compareContent,
            r'ApplicationConfigDiffResponse',
            'compareContent',
          ),
          compareVersion: compareVersion,
          currentContent: BuiltValueNullFieldError.checkNotNull(
            currentContent,
            r'ApplicationConfigDiffResponse',
            'currentContent',
          ),
          currentVersion: BuiltValueNullFieldError.checkNotNull(
            currentVersion,
            r'ApplicationConfigDiffResponse',
            'currentVersion',
          ),
          fileId: BuiltValueNullFieldError.checkNotNull(
            fileId,
            r'ApplicationConfigDiffResponse',
            'fileId',
          ),
          sensitive: BuiltValueNullFieldError.checkNotNull(
            sensitive,
            r'ApplicationConfigDiffResponse',
            'sensitive',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
