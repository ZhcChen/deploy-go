//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'application_config_file_response.g.dart';

/// ApplicationConfigFileResponse
///
/// Properties:
/// * [applicationId]
/// * [bindingId]
/// * [content]
/// * [currentDigest]
/// * [currentVersion]
/// * [deletedAt]
/// * [delivery]
/// * [deployPath]
/// * [description]
/// * [editable]
/// * [format]
/// * [id]
/// * [label]
/// * [language]
/// * [path]
/// * [recommendedChanges]
/// * [role]
/// * [sensitive]
/// * [status]
/// * [templateSourceDigest]
/// * [updatedAt]
/// * [version]
@BuiltValue()
abstract class ApplicationConfigFileResponse implements Built<ApplicationConfigFileResponse, ApplicationConfigFileResponseBuilder> {
  @BuiltValueField(wireName: r'application_id')
  String get applicationId;

  @BuiltValueField(wireName: r'binding_id')
  String get bindingId;

  @BuiltValueField(wireName: r'content')
  String? get content;

  @BuiltValueField(wireName: r'current_digest')
  String? get currentDigest;

  @BuiltValueField(wireName: r'current_version')
  int get currentVersion;

  @BuiltValueField(wireName: r'deleted_at')
  String? get deletedAt;

  @BuiltValueField(wireName: r'delivery')
  String get delivery;

  @BuiltValueField(wireName: r'deploy_path')
  String? get deployPath;

  @BuiltValueField(wireName: r'description')
  String get description;

  @BuiltValueField(wireName: r'editable')
  bool get editable;

  @BuiltValueField(wireName: r'format')
  String get format;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'label')
  String get label;

  @BuiltValueField(wireName: r'language')
  String get language;

  @BuiltValueField(wireName: r'path')
  String get path;

  @BuiltValueField(wireName: r'recommended_changes')
  String get recommendedChanges;

  @BuiltValueField(wireName: r'role')
  String get role;

  @BuiltValueField(wireName: r'sensitive')
  bool get sensitive;

  @BuiltValueField(wireName: r'status')
  String get status;

  @BuiltValueField(wireName: r'template_source_digest')
  String? get templateSourceDigest;

  @BuiltValueField(wireName: r'updated_at')
  String get updatedAt;

  @BuiltValueField(wireName: r'version')
  int get version;

  ApplicationConfigFileResponse._();

  factory ApplicationConfigFileResponse([void updates(ApplicationConfigFileResponseBuilder b)]) = _$ApplicationConfigFileResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ApplicationConfigFileResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ApplicationConfigFileResponse> get serializer => _$ApplicationConfigFileResponseSerializer();
}

class _$ApplicationConfigFileResponseSerializer implements PrimitiveSerializer<ApplicationConfigFileResponse> {
  @override
  final Iterable<Type> types = const [ApplicationConfigFileResponse, _$ApplicationConfigFileResponse];

  @override
  final String wireName = r'ApplicationConfigFileResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ApplicationConfigFileResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'application_id';
    yield serializers.serialize(
      object.applicationId,
      specifiedType: const FullType(String),
    );
    yield r'binding_id';
    yield serializers.serialize(
      object.bindingId,
      specifiedType: const FullType(String),
    );
    if (object.content != null) {
      yield r'content';
      yield serializers.serialize(
        object.content,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.currentDigest != null) {
      yield r'current_digest';
      yield serializers.serialize(
        object.currentDigest,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'current_version';
    yield serializers.serialize(
      object.currentVersion,
      specifiedType: const FullType(int),
    );
    if (object.deletedAt != null) {
      yield r'deleted_at';
      yield serializers.serialize(
        object.deletedAt,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'delivery';
    yield serializers.serialize(
      object.delivery,
      specifiedType: const FullType(String),
    );
    if (object.deployPath != null) {
      yield r'deploy_path';
      yield serializers.serialize(
        object.deployPath,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'description';
    yield serializers.serialize(
      object.description,
      specifiedType: const FullType(String),
    );
    yield r'editable';
    yield serializers.serialize(
      object.editable,
      specifiedType: const FullType(bool),
    );
    yield r'format';
    yield serializers.serialize(
      object.format,
      specifiedType: const FullType(String),
    );
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    yield r'label';
    yield serializers.serialize(
      object.label,
      specifiedType: const FullType(String),
    );
    yield r'language';
    yield serializers.serialize(
      object.language,
      specifiedType: const FullType(String),
    );
    yield r'path';
    yield serializers.serialize(
      object.path,
      specifiedType: const FullType(String),
    );
    yield r'recommended_changes';
    yield serializers.serialize(
      object.recommendedChanges,
      specifiedType: const FullType(String),
    );
    yield r'role';
    yield serializers.serialize(
      object.role,
      specifiedType: const FullType(String),
    );
    yield r'sensitive';
    yield serializers.serialize(
      object.sensitive,
      specifiedType: const FullType(bool),
    );
    yield r'status';
    yield serializers.serialize(
      object.status,
      specifiedType: const FullType(String),
    );
    if (object.templateSourceDigest != null) {
      yield r'template_source_digest';
      yield serializers.serialize(
        object.templateSourceDigest,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'updated_at';
    yield serializers.serialize(
      object.updatedAt,
      specifiedType: const FullType(String),
    );
    yield r'version';
    yield serializers.serialize(
      object.version,
      specifiedType: const FullType(int),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ApplicationConfigFileResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ApplicationConfigFileResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'application_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.applicationId = valueDes;
          break;
        case r'binding_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.bindingId = valueDes;
          break;
        case r'content':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.content = valueDes;
          break;
        case r'current_digest':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.currentDigest = valueDes;
          break;
        case r'current_version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.currentVersion = valueDes;
          break;
        case r'deleted_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.deletedAt = valueDes;
          break;
        case r'delivery':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.delivery = valueDes;
          break;
        case r'deploy_path':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.deployPath = valueDes;
          break;
        case r'description':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.description = valueDes;
          break;
        case r'editable':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.editable = valueDes;
          break;
        case r'format':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.format = valueDes;
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'label':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.label = valueDes;
          break;
        case r'language':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.language = valueDes;
          break;
        case r'path':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.path = valueDes;
          break;
        case r'recommended_changes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.recommendedChanges = valueDes;
          break;
        case r'role':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.role = valueDes;
          break;
        case r'sensitive':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.sensitive = valueDes;
          break;
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.status = valueDes;
          break;
        case r'template_source_digest':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.templateSourceDigest = valueDes;
          break;
        case r'updated_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.updatedAt = valueDes;
          break;
        case r'version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.version = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ApplicationConfigFileResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ApplicationConfigFileResponseBuilder();
    final serializedList = (serialized as Iterable<Object?>).toList();
    final unhandled = <Object?>[];
    _deserializeProperties(
      serializers,
      serialized,
      specifiedType: specifiedType,
      serializedList: serializedList,
      unhandled: unhandled,
      result: result,
    );
    return result.build();
  }
}
