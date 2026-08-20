// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'application_template_file_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ApplicationTemplateFileResponse
    extends ApplicationTemplateFileResponse {
  @override
  final String? content;
  @override
  final String delivery;
  @override
  final String? deployPath;
  @override
  final String description;
  @override
  final String digest;
  @override
  final bool editable;
  @override
  final String format;
  @override
  final String label;
  @override
  final String language;
  @override
  final String path;
  @override
  final String recommendedChanges;
  @override
  final String role;
  @override
  final bool sensitive;

  factory _$ApplicationTemplateFileResponse([
    void Function(ApplicationTemplateFileResponseBuilder)? updates,
  ]) => (ApplicationTemplateFileResponseBuilder()..update(updates))._build();

  _$ApplicationTemplateFileResponse._({
    this.content,
    required this.delivery,
    this.deployPath,
    required this.description,
    required this.digest,
    required this.editable,
    required this.format,
    required this.label,
    required this.language,
    required this.path,
    required this.recommendedChanges,
    required this.role,
    required this.sensitive,
  }) : super._();
  @override
  ApplicationTemplateFileResponse rebuild(
    void Function(ApplicationTemplateFileResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ApplicationTemplateFileResponseBuilder toBuilder() =>
      ApplicationTemplateFileResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ApplicationTemplateFileResponse &&
        content == other.content &&
        delivery == other.delivery &&
        deployPath == other.deployPath &&
        description == other.description &&
        digest == other.digest &&
        editable == other.editable &&
        format == other.format &&
        label == other.label &&
        language == other.language &&
        path == other.path &&
        recommendedChanges == other.recommendedChanges &&
        role == other.role &&
        sensitive == other.sensitive;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, content.hashCode);
    _$hash = $jc(_$hash, delivery.hashCode);
    _$hash = $jc(_$hash, deployPath.hashCode);
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, digest.hashCode);
    _$hash = $jc(_$hash, editable.hashCode);
    _$hash = $jc(_$hash, format.hashCode);
    _$hash = $jc(_$hash, label.hashCode);
    _$hash = $jc(_$hash, language.hashCode);
    _$hash = $jc(_$hash, path.hashCode);
    _$hash = $jc(_$hash, recommendedChanges.hashCode);
    _$hash = $jc(_$hash, role.hashCode);
    _$hash = $jc(_$hash, sensitive.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ApplicationTemplateFileResponse')
          ..add('content', content)
          ..add('delivery', delivery)
          ..add('deployPath', deployPath)
          ..add('description', description)
          ..add('digest', digest)
          ..add('editable', editable)
          ..add('format', format)
          ..add('label', label)
          ..add('language', language)
          ..add('path', path)
          ..add('recommendedChanges', recommendedChanges)
          ..add('role', role)
          ..add('sensitive', sensitive))
        .toString();
  }
}

class ApplicationTemplateFileResponseBuilder
    implements
        Builder<
          ApplicationTemplateFileResponse,
          ApplicationTemplateFileResponseBuilder
        > {
  _$ApplicationTemplateFileResponse? _$v;

  String? _content;
  String? get content => _$this._content;
  set content(String? content) => _$this._content = content;

  String? _delivery;
  String? get delivery => _$this._delivery;
  set delivery(String? delivery) => _$this._delivery = delivery;

  String? _deployPath;
  String? get deployPath => _$this._deployPath;
  set deployPath(String? deployPath) => _$this._deployPath = deployPath;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  String? _digest;
  String? get digest => _$this._digest;
  set digest(String? digest) => _$this._digest = digest;

  bool? _editable;
  bool? get editable => _$this._editable;
  set editable(bool? editable) => _$this._editable = editable;

  String? _format;
  String? get format => _$this._format;
  set format(String? format) => _$this._format = format;

  String? _label;
  String? get label => _$this._label;
  set label(String? label) => _$this._label = label;

  String? _language;
  String? get language => _$this._language;
  set language(String? language) => _$this._language = language;

  String? _path;
  String? get path => _$this._path;
  set path(String? path) => _$this._path = path;

  String? _recommendedChanges;
  String? get recommendedChanges => _$this._recommendedChanges;
  set recommendedChanges(String? recommendedChanges) =>
      _$this._recommendedChanges = recommendedChanges;

  String? _role;
  String? get role => _$this._role;
  set role(String? role) => _$this._role = role;

  bool? _sensitive;
  bool? get sensitive => _$this._sensitive;
  set sensitive(bool? sensitive) => _$this._sensitive = sensitive;

  ApplicationTemplateFileResponseBuilder() {
    ApplicationTemplateFileResponse._defaults(this);
  }

  ApplicationTemplateFileResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _content = $v.content;
      _delivery = $v.delivery;
      _deployPath = $v.deployPath;
      _description = $v.description;
      _digest = $v.digest;
      _editable = $v.editable;
      _format = $v.format;
      _label = $v.label;
      _language = $v.language;
      _path = $v.path;
      _recommendedChanges = $v.recommendedChanges;
      _role = $v.role;
      _sensitive = $v.sensitive;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ApplicationTemplateFileResponse other) {
    _$v = other as _$ApplicationTemplateFileResponse;
  }

  @override
  void update(void Function(ApplicationTemplateFileResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ApplicationTemplateFileResponse build() => _build();

  _$ApplicationTemplateFileResponse _build() {
    final _$result =
        _$v ??
        _$ApplicationTemplateFileResponse._(
          content: content,
          delivery: BuiltValueNullFieldError.checkNotNull(
            delivery,
            r'ApplicationTemplateFileResponse',
            'delivery',
          ),
          deployPath: deployPath,
          description: BuiltValueNullFieldError.checkNotNull(
            description,
            r'ApplicationTemplateFileResponse',
            'description',
          ),
          digest: BuiltValueNullFieldError.checkNotNull(
            digest,
            r'ApplicationTemplateFileResponse',
            'digest',
          ),
          editable: BuiltValueNullFieldError.checkNotNull(
            editable,
            r'ApplicationTemplateFileResponse',
            'editable',
          ),
          format: BuiltValueNullFieldError.checkNotNull(
            format,
            r'ApplicationTemplateFileResponse',
            'format',
          ),
          label: BuiltValueNullFieldError.checkNotNull(
            label,
            r'ApplicationTemplateFileResponse',
            'label',
          ),
          language: BuiltValueNullFieldError.checkNotNull(
            language,
            r'ApplicationTemplateFileResponse',
            'language',
          ),
          path: BuiltValueNullFieldError.checkNotNull(
            path,
            r'ApplicationTemplateFileResponse',
            'path',
          ),
          recommendedChanges: BuiltValueNullFieldError.checkNotNull(
            recommendedChanges,
            r'ApplicationTemplateFileResponse',
            'recommendedChanges',
          ),
          role: BuiltValueNullFieldError.checkNotNull(
            role,
            r'ApplicationTemplateFileResponse',
            'role',
          ),
          sensitive: BuiltValueNullFieldError.checkNotNull(
            sensitive,
            r'ApplicationTemplateFileResponse',
            'sensitive',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
