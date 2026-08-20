// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'application_config_file_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ApplicationConfigFileResponse extends ApplicationConfigFileResponse {
  @override
  final String applicationId;
  @override
  final String bindingId;
  @override
  final String? content;
  @override
  final String? currentDigest;
  @override
  final int currentVersion;
  @override
  final String? deletedAt;
  @override
  final String delivery;
  @override
  final String? deployPath;
  @override
  final String description;
  @override
  final bool editable;
  @override
  final String format;
  @override
  final String id;
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
  @override
  final String status;
  @override
  final String? templateSourceDigest;
  @override
  final String updatedAt;
  @override
  final int version;

  factory _$ApplicationConfigFileResponse([
    void Function(ApplicationConfigFileResponseBuilder)? updates,
  ]) => (ApplicationConfigFileResponseBuilder()..update(updates))._build();

  _$ApplicationConfigFileResponse._({
    required this.applicationId,
    required this.bindingId,
    this.content,
    this.currentDigest,
    required this.currentVersion,
    this.deletedAt,
    required this.delivery,
    this.deployPath,
    required this.description,
    required this.editable,
    required this.format,
    required this.id,
    required this.label,
    required this.language,
    required this.path,
    required this.recommendedChanges,
    required this.role,
    required this.sensitive,
    required this.status,
    this.templateSourceDigest,
    required this.updatedAt,
    required this.version,
  }) : super._();
  @override
  ApplicationConfigFileResponse rebuild(
    void Function(ApplicationConfigFileResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ApplicationConfigFileResponseBuilder toBuilder() =>
      ApplicationConfigFileResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ApplicationConfigFileResponse &&
        applicationId == other.applicationId &&
        bindingId == other.bindingId &&
        content == other.content &&
        currentDigest == other.currentDigest &&
        currentVersion == other.currentVersion &&
        deletedAt == other.deletedAt &&
        delivery == other.delivery &&
        deployPath == other.deployPath &&
        description == other.description &&
        editable == other.editable &&
        format == other.format &&
        id == other.id &&
        label == other.label &&
        language == other.language &&
        path == other.path &&
        recommendedChanges == other.recommendedChanges &&
        role == other.role &&
        sensitive == other.sensitive &&
        status == other.status &&
        templateSourceDigest == other.templateSourceDigest &&
        updatedAt == other.updatedAt &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, applicationId.hashCode);
    _$hash = $jc(_$hash, bindingId.hashCode);
    _$hash = $jc(_$hash, content.hashCode);
    _$hash = $jc(_$hash, currentDigest.hashCode);
    _$hash = $jc(_$hash, currentVersion.hashCode);
    _$hash = $jc(_$hash, deletedAt.hashCode);
    _$hash = $jc(_$hash, delivery.hashCode);
    _$hash = $jc(_$hash, deployPath.hashCode);
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, editable.hashCode);
    _$hash = $jc(_$hash, format.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, label.hashCode);
    _$hash = $jc(_$hash, language.hashCode);
    _$hash = $jc(_$hash, path.hashCode);
    _$hash = $jc(_$hash, recommendedChanges.hashCode);
    _$hash = $jc(_$hash, role.hashCode);
    _$hash = $jc(_$hash, sensitive.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, templateSourceDigest.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ApplicationConfigFileResponse')
          ..add('applicationId', applicationId)
          ..add('bindingId', bindingId)
          ..add('content', content)
          ..add('currentDigest', currentDigest)
          ..add('currentVersion', currentVersion)
          ..add('deletedAt', deletedAt)
          ..add('delivery', delivery)
          ..add('deployPath', deployPath)
          ..add('description', description)
          ..add('editable', editable)
          ..add('format', format)
          ..add('id', id)
          ..add('label', label)
          ..add('language', language)
          ..add('path', path)
          ..add('recommendedChanges', recommendedChanges)
          ..add('role', role)
          ..add('sensitive', sensitive)
          ..add('status', status)
          ..add('templateSourceDigest', templateSourceDigest)
          ..add('updatedAt', updatedAt)
          ..add('version', version))
        .toString();
  }
}

class ApplicationConfigFileResponseBuilder
    implements
        Builder<
          ApplicationConfigFileResponse,
          ApplicationConfigFileResponseBuilder
        > {
  _$ApplicationConfigFileResponse? _$v;

  String? _applicationId;
  String? get applicationId => _$this._applicationId;
  set applicationId(String? applicationId) =>
      _$this._applicationId = applicationId;

  String? _bindingId;
  String? get bindingId => _$this._bindingId;
  set bindingId(String? bindingId) => _$this._bindingId = bindingId;

  String? _content;
  String? get content => _$this._content;
  set content(String? content) => _$this._content = content;

  String? _currentDigest;
  String? get currentDigest => _$this._currentDigest;
  set currentDigest(String? currentDigest) =>
      _$this._currentDigest = currentDigest;

  int? _currentVersion;
  int? get currentVersion => _$this._currentVersion;
  set currentVersion(int? currentVersion) =>
      _$this._currentVersion = currentVersion;

  String? _deletedAt;
  String? get deletedAt => _$this._deletedAt;
  set deletedAt(String? deletedAt) => _$this._deletedAt = deletedAt;

  String? _delivery;
  String? get delivery => _$this._delivery;
  set delivery(String? delivery) => _$this._delivery = delivery;

  String? _deployPath;
  String? get deployPath => _$this._deployPath;
  set deployPath(String? deployPath) => _$this._deployPath = deployPath;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  bool? _editable;
  bool? get editable => _$this._editable;
  set editable(bool? editable) => _$this._editable = editable;

  String? _format;
  String? get format => _$this._format;
  set format(String? format) => _$this._format = format;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

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

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  String? _templateSourceDigest;
  String? get templateSourceDigest => _$this._templateSourceDigest;
  set templateSourceDigest(String? templateSourceDigest) =>
      _$this._templateSourceDigest = templateSourceDigest;

  String? _updatedAt;
  String? get updatedAt => _$this._updatedAt;
  set updatedAt(String? updatedAt) => _$this._updatedAt = updatedAt;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  ApplicationConfigFileResponseBuilder() {
    ApplicationConfigFileResponse._defaults(this);
  }

  ApplicationConfigFileResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _applicationId = $v.applicationId;
      _bindingId = $v.bindingId;
      _content = $v.content;
      _currentDigest = $v.currentDigest;
      _currentVersion = $v.currentVersion;
      _deletedAt = $v.deletedAt;
      _delivery = $v.delivery;
      _deployPath = $v.deployPath;
      _description = $v.description;
      _editable = $v.editable;
      _format = $v.format;
      _id = $v.id;
      _label = $v.label;
      _language = $v.language;
      _path = $v.path;
      _recommendedChanges = $v.recommendedChanges;
      _role = $v.role;
      _sensitive = $v.sensitive;
      _status = $v.status;
      _templateSourceDigest = $v.templateSourceDigest;
      _updatedAt = $v.updatedAt;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ApplicationConfigFileResponse other) {
    _$v = other as _$ApplicationConfigFileResponse;
  }

  @override
  void update(void Function(ApplicationConfigFileResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ApplicationConfigFileResponse build() => _build();

  _$ApplicationConfigFileResponse _build() {
    final _$result =
        _$v ??
        _$ApplicationConfigFileResponse._(
          applicationId: BuiltValueNullFieldError.checkNotNull(
            applicationId,
            r'ApplicationConfigFileResponse',
            'applicationId',
          ),
          bindingId: BuiltValueNullFieldError.checkNotNull(
            bindingId,
            r'ApplicationConfigFileResponse',
            'bindingId',
          ),
          content: content,
          currentDigest: currentDigest,
          currentVersion: BuiltValueNullFieldError.checkNotNull(
            currentVersion,
            r'ApplicationConfigFileResponse',
            'currentVersion',
          ),
          deletedAt: deletedAt,
          delivery: BuiltValueNullFieldError.checkNotNull(
            delivery,
            r'ApplicationConfigFileResponse',
            'delivery',
          ),
          deployPath: deployPath,
          description: BuiltValueNullFieldError.checkNotNull(
            description,
            r'ApplicationConfigFileResponse',
            'description',
          ),
          editable: BuiltValueNullFieldError.checkNotNull(
            editable,
            r'ApplicationConfigFileResponse',
            'editable',
          ),
          format: BuiltValueNullFieldError.checkNotNull(
            format,
            r'ApplicationConfigFileResponse',
            'format',
          ),
          id: BuiltValueNullFieldError.checkNotNull(
            id,
            r'ApplicationConfigFileResponse',
            'id',
          ),
          label: BuiltValueNullFieldError.checkNotNull(
            label,
            r'ApplicationConfigFileResponse',
            'label',
          ),
          language: BuiltValueNullFieldError.checkNotNull(
            language,
            r'ApplicationConfigFileResponse',
            'language',
          ),
          path: BuiltValueNullFieldError.checkNotNull(
            path,
            r'ApplicationConfigFileResponse',
            'path',
          ),
          recommendedChanges: BuiltValueNullFieldError.checkNotNull(
            recommendedChanges,
            r'ApplicationConfigFileResponse',
            'recommendedChanges',
          ),
          role: BuiltValueNullFieldError.checkNotNull(
            role,
            r'ApplicationConfigFileResponse',
            'role',
          ),
          sensitive: BuiltValueNullFieldError.checkNotNull(
            sensitive,
            r'ApplicationConfigFileResponse',
            'sensitive',
          ),
          status: BuiltValueNullFieldError.checkNotNull(
            status,
            r'ApplicationConfigFileResponse',
            'status',
          ),
          templateSourceDigest: templateSourceDigest,
          updatedAt: BuiltValueNullFieldError.checkNotNull(
            updatedAt,
            r'ApplicationConfigFileResponse',
            'updatedAt',
          ),
          version: BuiltValueNullFieldError.checkNotNull(
            version,
            r'ApplicationConfigFileResponse',
            'version',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
